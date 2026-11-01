use regex::Regex;
use std::{
    cmp::Ordering,
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Episode {
    pub season: Option<u32>,
    pub number: u32,
}

static PATTERNS: LazyLock<Result<Vec<Regex>, regex::Error>> =
    LazyLock::new(|| {
        [
        r"(?i)(?:^|[^a-z0-9])s(?P<season>\d{1,2})e(?P<ep>\d{1,4})(?:v\d+)?(?:$|[^0-9])",
        r"(?i)(?:^|[\s._\[\]-])(?:episode|ep|e)[\s._-]*(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\[\]-])",
        r"(?i)(?:^|\s)-\s*(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\[\]-])",
        r"(?i)^(?P<ep>\d{1,4})(?:v\d+)?$",
        r"(?i)(?:^|[\s._\[-])(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\]-])",
    ].into_iter().map(Regex::new).collect()
    });

pub fn episode_number(stem: &str) -> Option<Episode> {
    let patterns = PATTERNS.as_ref().ok()?;
    for (index, pattern) in patterns.iter().enumerate() {
        let mut offset = 0;
        let mut candidates = Vec::new();
        while let Some(capture) = pattern.captures_at(stem, offset) {
            let Some(number_match) = capture.name("ep") else {
                break;
            };
            offset = number_match.end();
            if let Ok(number) = number_match.as_str().parse::<u32>() {
                if index == 4
                    && ([360, 480, 576, 720, 1080, 1440, 2160, 4320].contains(&number)
                        || (1900..=2099).contains(&number))
                {
                    continue;
                }
                candidates.push(Episode {
                    season: capture.name("season").and_then(|m| m.as_str().parse().ok()),
                    number,
                });
            }
            if candidates.len() > 1 {
                return None;
            }
        }
        if candidates.len() == 1 {
            return candidates.first().copied();
        }
    }
    None
}

pub fn clean_title(title: &str) -> Result<String, String> {
    let title: String = title
        .chars()
        .map(|ch| {
            if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = title.trim_end_matches(['.', ' ']).to_owned();
    if title.chars().count() > 100 {
        return Err("Use an anime title of at most 100 characters.".into());
    }
    Ok(title)
}

pub fn output_name(input: &Path, title: &str, sequence: usize) -> Result<OsString, String> {
    if !title.is_empty() {
        return Ok(format!("{title} - Ep{sequence:02} [Wii].mp4").into());
    }
    let mut name = OsString::from(input.file_stem().ok_or("Invalid video filename.")?);
    name.push("_Wii.mp4");
    Ok(name)
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().peekable();
    let mut right = right.chars().peekable();
    while let (Some(&a), Some(&b)) = (left.peek(), right.peek()) {
        if a.is_ascii_digit() && b.is_ascii_digit() {
            let a: String = std::iter::from_fn(|| left.next_if(|c| c.is_ascii_digit())).collect();
            let b: String = std::iter::from_fn(|| right.next_if(|c| c.is_ascii_digit())).collect();
            let a = a.trim_start_matches('0');
            let b = b.trim_start_matches('0');
            let order = a.len().cmp(&b.len()).then_with(|| a.cmp(b));
            if order != Ordering::Equal {
                return order;
            }
        } else {
            let order = a.cmp(&b);
            if order != Ordering::Equal {
                return order;
            }
            left.next();
            right.next();
        }
    }
    left.count().cmp(&right.count())
}

pub fn compare_videos(left: &Path, right: &Path) -> Ordering {
    let left_stem = left
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let right_stem = right
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let left_ep = episode_number(&left_stem);
    let right_ep = episode_number(&right_stem);
    match (left_ep, right_ep) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| compare_filenames(left, right))
}

fn compare_filenames(left: &Path, right: &Path) -> Ordering {
    let left_name = left
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let right_name = right
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    natural_cmp(&left_name, &right_name).then_with(|| left.cmp(right))
}

struct SubtitleIndex {
    stems: HashMap<String, Vec<usize>>,
    episodes: HashMap<u32, Vec<(usize, Option<u32>)>>,
}
impl SubtitleIndex {
    fn new(subtitles: &[PathBuf]) -> Self {
        let mut index = Self {
            stems: HashMap::new(),
            episodes: HashMap::new(),
        };
        for (position, path) in subtitles.iter().enumerate() {
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let stem = stem.to_string_lossy().to_lowercase();
            index.stems.entry(stem.clone()).or_default().push(position);
            if let Some(episode) = episode_number(&stem) {
                index
                    .episodes
                    .entry(episode.number)
                    .or_default()
                    .push((position, episode.season));
            }
        }
        index
    }
    fn candidate(&self, video: &Path) -> Result<Option<usize>, String> {
        let stem = video
            .file_stem()
            .ok_or("Invalid video filename.")?
            .to_string_lossy()
            .to_lowercase();
        if let Some(exact) = self.stems.get(&stem) {
            return match exact.as_slice() {
                [position] => Ok(Some(*position)),
                _ => Err(format!("Multiple subtitles exactly match {stem}.")),
            };
        }
        let Some(episode) = episode_number(&stem) else {
            return Ok(None);
        };
        let mut candidates = self
            .episodes
            .get(&episode.number)
            .into_iter()
            .flatten()
            .filter(|(_, season)| {
                episode.season.is_none() || season.is_none() || episode.season == *season
            });
        let first = candidates.next().map(|(position, _)| *position);
        if candidates.next().is_some() {
            return Err(format!(
                "Multiple subtitles match episode {} of {stem}.",
                episode.number
            ));
        }
        Ok(first)
    }
}

pub struct SubtitlePairing {
    /// One entry per video in the caller's queue order.
    pub matches: Vec<Option<PathBuf>>,
    pub warnings: Vec<String>,
}

pub fn pair_subtitles(videos: &[PathBuf], subtitles: &[PathBuf]) -> SubtitlePairing {
    if subtitles.is_empty() {
        return SubtitlePairing {
            matches: vec![None; videos.len()],
            warnings: if videos.is_empty() {
                Vec::new()
            } else {
                vec![format!(
                    "Subtitle file-count mismatch: {} videos, 0 subtitles; {} unpaired video(s). Encoding without external subtitles.",
                    videos.len(),
                    videos.len()
                )]
            },
        };
    }
    let index = SubtitleIndex::new(subtitles);
    let mut proposals = Vec::with_capacity(videos.len());
    let mut claims = vec![0usize; subtitles.len()];
    let mut ambiguous = 0;
    for video in videos {
        let candidate = match index.candidate(video) {
            Ok(candidate) => candidate,
            Err(_) => {
                ambiguous += 1;
                None
            }
        };
        if let Some(position) = candidate {
            if let Some(count) = claims.get_mut(position) {
                *count += 1;
            }
        }
        proposals.push(candidate);
    }
    let mut matches = vec![None; videos.len()];
    let mut used = vec![false; subtitles.len()];
    for (video, proposal) in proposals.into_iter().enumerate() {
        if let Some(subtitle) = proposal {
            if claims.get(subtitle) == Some(&1) {
                if let (Some(destination), Some(source), Some(in_use)) = (
                    matches.get_mut(video),
                    subtitles.get(subtitle),
                    used.get_mut(subtitle),
                ) {
                    *destination = Some(source.clone());
                    *in_use = true;
                }
            } else {
                ambiguous += 1;
            }
        }
    }
    // Indices come from these unchanged slices. Keep absolute sorted positions;
    // never shift leftovers or steal a confident match.
    let mut video_order: Vec<_> = (0..videos.len()).collect();
    let mut subtitle_order: Vec<_> = (0..subtitles.len()).collect();
    video_order.sort_by(|a, b| compare_filenames(&videos[*a], &videos[*b]));
    subtitle_order.sort_by(|a, b| compare_filenames(&subtitles[*a], &subtitles[*b]));
    let mut fallback = 0;
    for (rank, video) in video_order.into_iter().enumerate() {
        if matches.get(video).is_none_or(|pair| pair.is_some()) {
            continue;
        }
        let Some(subtitle) = subtitle_order.get(rank).copied() else {
            continue;
        };
        if used.get(subtitle) != Some(&false) {
            continue;
        }
        if let (Some(destination), Some(source), Some(in_use)) = (
            matches.get_mut(video),
            subtitles.get(subtitle),
            used.get_mut(subtitle),
        ) {
            *destination = Some(source.clone());
            *in_use = true;
            fallback += 1;
        }
    }
    let unmatched = matches.iter().filter(|pair| pair.is_none()).count();
    let unused = used.iter().filter(|in_use| !**in_use).count();
    let mut warnings = Vec::new();
    if fallback > 0 {
        warnings.push(format!("Used alphanumeric index fallback for {fallback} video(s) without a unique filename/episode match. Verify the subtitle ordering before a full batch."));
    }
    if ambiguous > 0 {
        warnings.push(format!("{ambiguous} video(s) had ambiguous or competing subtitle matches; only unclaimed same-index subtitles were eligible for fallback."));
    }
    if videos.len() != subtitles.len() && (unmatched > 0 || unused > 0) {
        warnings.push(format!("Subtitle file-count mismatch: {} videos, {} subtitles; {unmatched} unpaired video(s), {unused} unused subtitle(s). Episode matching and strict index fallback could not resolve all orphans. Unpaired videos will encode without external subtitles.", videos.len(), subtitles.len()));
    } else if unmatched > 0 && !subtitles.is_empty() {
        warnings.push(format!("{unmatched} video(s) could not be paired without reusing a subtitle reserved by a confident match; they will encode without external subtitles."));
    }
    SubtitlePairing { matches, warnings }
}

#[cfg(test)]
fn match_subtitle(video: &Path, subtitles: &[PathBuf]) -> Result<Option<PathBuf>, String> {
    SubtitleIndex::new(subtitles)
        .candidate(video)
        .map(|position| position.and_then(|position| subtitles.get(position).cloned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }
    #[test]
    fn episode_tokens_match_across_names_without_matching_resolution_or_substrings() {
        let videos = paths(&[
            "[Subs] Keroro 01 (1080p).mkv",
            "[Subs] Keroro 10 (1080p).mkv",
        ]);
        let subtitles = paths(&[
            "Different release Ep101.ass",
            "Other title - 10.srt",
            "Fansub episode 01.vtt",
        ]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches,
            vec![Some(subtitles[2].clone()), Some(subtitles[1].clone())]
        );
        assert_eq!(episode_number("Unlabelled 01 02"), None);
        assert_eq!(episode_number("Movie 2026 1080"), None);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("2 videos, 3 subtitles"))
        );
    }
    #[test]
    fn fallback_uses_natural_sorted_indices_and_restores_caller_order() {
        let videos = paths(&["Film10.mkv", "Film2.mkv", "Film1.mkv"]);
        let subtitles = paths(&["Track C.ass", "Track A.srt", "Track B.vtt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches,
            vec![
                Some(subtitles[0].clone()),
                Some(subtitles[2].clone()),
                Some(subtitles[1].clone())
            ]
        );
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("index fallback for 3"))
        );
    }
    #[test]
    fn fallback_never_steals_a_confident_match_or_shifts_remaining_indices() {
        let videos = paths(&["Unknown.mkv", "Known E02.mkv"]);
        let subtitles = paths(&["A unnumbered.ass", "Z episode02.srt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(pairing.matches, vec![None, Some(subtitles[1].clone())]);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("without reusing"))
        );
    }
    #[test]
    fn count_mismatch_warns_and_leaves_video_only_orphans() {
        let videos = paths(&["Unnumbered B.mkv", "Unnumbered A.mkv"]);
        let subtitles = paths(&["Subtitle.ass"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(pairing.matches, vec![None, Some(subtitles[0].clone())]);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("2 videos, 1 subtitles"))
        );
        assert!(
            pair_subtitles(&videos, &[])
                .warnings
                .iter()
                .any(|warning| warning.contains("0 subtitles"))
        );
    }
    #[test]
    fn competing_primary_matches_never_reuse_the_same_subtitle() {
        let videos = paths(&["A E01.mkv", "B E01.mkv"]);
        let subtitles = paths(&["Other episode01.srt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches.iter().filter(|pair| pair.is_some()).count(),
            1
        );
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("competing"))
        );
    }
    #[test]
    fn matching_does_not_confuse_seasons_resolutions_or_ambiguous_files() {
        assert_eq!(
            episode_number("[Group] Keroro - 03 [1080p]"),
            Some(Episode {
                season: None,
                number: 3
            })
        );
        assert_eq!(
            episode_number("Keroro S02E03"),
            Some(Episode {
                season: Some(2),
                number: 3
            })
        );
        assert_eq!(episode_number("Movie 2026 1080"), None);
        let files = vec![
            PathBuf::from("Other S01E03.srt"),
            PathBuf::from("Other S02E03.ass"),
        ];
        assert_eq!(
            match_subtitle(Path::new("Keroro S02E03.mkv"), &files).expect("match"),
            Some(files[1].clone())
        );
        assert!(match_subtitle(Path::new("Keroro E03.mkv"), &files).is_err());
        let exact = vec![
            PathBuf::from("Keroro E03.ass"),
            PathBuf::from("Other E03.srt"),
        ];
        assert_eq!(
            match_subtitle(Path::new("Keroro E03.mkv"), &exact).expect("exact"),
            Some(exact[0].clone())
        );
    }
    #[test]
    fn titles_and_natural_order_produce_safe_sequential_names() {
        assert_eq!(
            clean_title("  Sgt: Keroro / ").expect("title"),
            "Sgt Keroro"
        );
        assert_eq!(
            output_name(Path::new("Episode10.mkv"), "Sgt Keroro", 1).expect("name"),
            OsString::from("Sgt Keroro - Ep01 [Wii].mp4")
        );
        assert_eq!(
            output_name(Path::new("Episode10.mkv"), "", 1).expect("name"),
            OsString::from("Episode10_Wii.mp4")
        );
        assert_eq!(
            compare_videos(Path::new("Ep2.mkv"), Path::new("Ep10.mkv")),
            Ordering::Less
        );
    }
}
