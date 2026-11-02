//! Validated SD geometry and SAR-aware, non-stretching filter construction.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EncoderPreference {
    #[default]
    Auto,
    Nvenc,
    Amf,
    Qsv,
    Cpu,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum AspectRatio {
    #[serde(rename = "auto")]
    Auto,
    #[default]
    #[serde(rename = "4:3")]
    Standard,
    #[serde(rename = "16:9")]
    Wide,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ResolutionPreset {
    #[default]
    #[serde(rename = "640x480")]
    Standard,
    #[serde(rename = "848x480", alias = "854x480")]
    Wide,
    #[serde(rename = "720x480")]
    Anamorphic,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct VideoSettings {
    pub aspect_ratio: AspectRatio,
    pub resolution_preset: ResolutionPreset,
    pub custom_width: u32,
    pub custom_height: u32,
    pub auto_crop: bool,
    pub encoder_override: EncoderPreference,
}
impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            aspect_ratio: AspectRatio::Standard,
            resolution_preset: ResolutionPreset::Standard,
            custom_width: 640,
            custom_height: 480,
            auto_crop: false,
            encoder_override: EncoderPreference::Auto,
        }
    }
}

impl VideoSettings {
    pub fn dimensions(&self) -> (u32, u32) {
        match self.resolution_preset {
            ResolutionPreset::Standard => (640, 480),
            ResolutionPreset::Wide => (848, 480),
            ResolutionPreset::Anamorphic => (720, 480),
            ResolutionPreset::Custom => (
                round_even(self.custom_width),
                round_even(self.custom_height),
            ),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let (w, h) = self.dimensions();
        if !(2..=854).contains(&w) || !(2..=480).contains(&h) || w % 2 != 0 || h % 2 != 0 {
            return Err("Use even output dimensions: width 2–854 and height 2–480.".into());
        }
        Ok(())
    }
    pub fn max_fps(&self) -> Result<u32, String> {
        self.validate()?;
        let (w, h) = self.dimensions();
        // Retain the conservative decoding workload budget even with Level 3.1.
        // 848x480 stays at 25 fps; 640x480 and 720x480 allow at most 30 fps.
        Ok((40500 / (w.div_ceil(16) * h.div_ceil(16))).min(30))
    }
    pub fn display_ratio(&self) -> Option<f64> {
        match self.aspect_ratio {
            AspectRatio::Auto => None,
            AspectRatio::Standard => Some(4.0 / 3.0),
            AspectRatio::Wide => Some(16.0 / 9.0),
        }
    }
    pub fn normalized(&self) -> Self {
        let mut value = self.clone();
        if value.resolution_preset != ResolutionPreset::Custom {
            value.custom_width = 640;
            value.custom_height = 480;
        } else {
            value.custom_width = round_even(value.custom_width);
            value.custom_height = round_even(value.custom_height);
        }
        value
    }
    pub fn filter(&self) -> Result<String, String> {
        self.validate()?;
        let (w, h) = self.dimensions();
        let ratio = match self.aspect_ratio {
            // scale's default SAR adjustment preserves the source DAR, even
            // when the chosen storage raster is not square-pixel widescreen.
            AspectRatio::Auto => return Ok(format!("scale={w}:{h},format=yuv420p")),
            AspectRatio::Standard => "4/3",
            AspectRatio::Wide => "16/9",
        };
        // Fit source display aspect (dar includes source SAR) into the target
        // raster after accounting for its target sample aspect ratio.
        let sar = format!("({ratio})*{h}/{w}");
        Ok(format!(
            "scale=w='max(2,trunc(min({w},{h}*dar/({sar}))/2)*2)':h='max(2,trunc(min({h},{w}*({sar})/dar)/2)*2)',pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setdar={ratio}:max=10000,format=yuv420p"
        ))
    }
    pub fn filter_with_crop(&self, crop: Option<crate::crop::CropRect>) -> Result<String, String> {
        let filter = self.filter()?;
        Ok(match crop {
            Some(rect) => format!("{},{filter}", rect.filter()),
            None => filter,
        })
    }
}

fn round_even(value: u32) -> u32 {
    value.saturating_add(1) / 2 * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn custom_geometry_survives_checkpoint_recovery() {
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("Ep03.mkv");
        std::fs::write(&input, []).expect("video");
        let geometry = VideoSettings {
            aspect_ratio: AspectRatio::Wide,
            resolution_preset: ResolutionPreset::Custom,
            custom_width: 704,
            custom_height: 400,
            auto_crop: true,
            encoder_override: EncoderPreference::Cpu,
        };
        let queue = crate::BatchPlan::prepare(
            "geometry".into(),
            crate::Settings {
                input_dir: input.to_string_lossy().into_owned(),
                output_dir: root.path().join("output").to_string_lossy().into_owned(),
                subtitle_dir: None,
                anime_title: "Keroro".into(),
                crf: 23,
                cpu_mode: crate::CpuMode::Maximum,
                video: geometry.clone(),
            },
        )
        .await
        .expect("queue");
        let store = crate::session::SessionStore::open(&root.path().join("state")).expect("store");
        store.begin(&queue).expect("checkpoint");
        drop(store);
        let store = crate::session::SessionStore::open(&root.path().join("state")).expect("reopen");
        let restored = store
            .resume_checked("geometry", "restored", &|| false)
            .expect("resume");
        assert_eq!(restored.settings().video, geometry);
        assert_eq!(
            restored.settings().video.filter().expect("filter"),
            queue.settings().video.filter().expect("original filter")
        );
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg/ffprobe paths"]
    async fn cpu_profile_encodes_wide_and_anamorphic_with_correct_metadata() {
        let tools = crate::Tools {
            ffmpeg: std::env::var_os("KERORO_TEST_FFMPEG")
                .expect("ffmpeg")
                .into(),
            ffprobe: std::env::var_os("KERORO_TEST_FFPROBE")
                .expect("ffprobe")
                .into(),
        };
        let root = tempfile::tempdir().expect("fixture");
        for preset in [ResolutionPreset::Wide, ResolutionPreset::Anamorphic] {
            let geometry = VideoSettings {
                aspect_ratio: AspectRatio::Wide,
                resolution_preset: preset,
                ..Default::default()
            };
            let file = root.path().join(format!("{:?}.mp4", preset));
            let mut command = crate::process(&tools.ffmpeg);
            command.args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:size=720x480:rate=60:duration=0.4,setsar=32/27",
            ]);
            crate::profile::configure(
                &mut command,
                &geometry.filter().expect("filter"),
                crate::hardware::VideoEncoder::Cpu,
                23,
                "1",
                geometry.max_fps().expect("fps"),
            );
            let result = command
                .args(["-frames:v", "6", "-an"])
                .arg(&file)
                .output()
                .await
                .expect("encode");
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            crate::validation::validate(
                &tools,
                &file,
                None,
                Some(false),
                &crate::Cancellation::new(),
                &std::sync::Arc::new(crate::ActiveProcess::default()),
                &geometry,
            )
            .await
            .expect("validated CPU output");
        }
    }

    #[test]
    fn sd_limits_even_dimensions_and_level_budget_are_enforced() {
        let mut v = VideoSettings::default();
        assert_eq!(v.max_fps().expect("standard"), 30);
        v.resolution_preset = ResolutionPreset::Wide;
        assert_eq!(v.max_fps().expect("wide"), 25);
        v.resolution_preset = ResolutionPreset::Anamorphic;
        assert_eq!(v.max_fps().expect("anamorphic"), 30);
        v.resolution_preset = ResolutionPreset::Custom;
        for (w, h) in [(0, 480), (856, 480), (640, 482)] {
            v.custom_width = w;
            v.custom_height = h;
            assert!(v.filter().is_err());
        }
        let legacy: VideoSettings = serde_json::from_str("{}").expect("legacy");
        assert_eq!(legacy, VideoSettings::default());
        let legacy: VideoSettings = serde_json::from_str(r#"{"aspectRatio":"16:9","resolutionPreset":"720x480","customWidth":704,"customHeight":400}"#).expect("v1 settings");
        assert!(!legacy.auto_crop);
        assert_eq!(legacy.encoder_override, EncoderPreference::Auto);
        assert_eq!(legacy.aspect_ratio, AspectRatio::Wide);
        let rounded = VideoSettings {
            resolution_preset: ResolutionPreset::Custom,
            custom_width: 705,
            custom_height: 479,
            ..Default::default()
        };
        assert_eq!(rounded.dimensions(), (706, 480));
        assert_eq!(rounded.normalized().custom_width, 706);
        let old_wide: VideoSettings =
            serde_json::from_str(r#"{"resolutionPreset":"854x480"}"#).expect("legacy widescreen");
        assert_eq!(old_wide.dimensions(), (848, 480));
        assert!(
            serde_json::to_string(&old_wide)
                .expect("serialize")
                .contains("848x480")
        );
    }
}
