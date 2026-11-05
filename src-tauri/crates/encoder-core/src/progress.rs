/// Timing updates require a changed integer percentage or ETA second.
#[derive(Default)]
pub struct ProgressGate {
    last: Option<Option<u8>>,
    last_eta: Option<Option<u64>>,
    last_metrics: Option<(std::time::Instant, Option<u64>, Option<String>)>,
}
impl ProgressGate {
    pub fn should_emit_metrics(&mut self, fps: Option<f64>, speed: Option<&str>) -> bool {
        self.metrics_at(fps, speed, std::time::Instant::now())
    }
    fn metrics_at(
        &mut self,
        fps: Option<f64>,
        speed: Option<&str>,
        now: std::time::Instant,
    ) -> bool {
        let fps = fps
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(f64::to_bits);
        if self
            .last_metrics
            .as_ref()
            .is_some_and(|(last, previous_fps, previous_speed)| {
                now.duration_since(*last) < std::time::Duration::from_secs(1)
                    || (*previous_fps == fps && previous_speed.as_deref() == speed)
            })
        {
            return false;
        }
        self.last_metrics = Some((now, fps, speed.map(str::to_owned)));
        true
    }
    pub fn should_emit(&mut self, percentage: Option<f64>) -> bool {
        let integer = percentage
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 100.0).floor() as u8);
        if self.last == Some(integer) {
            return false;
        }
        self.last = Some(integer);
        true
    }
    pub fn should_emit_eta(&mut self, seconds: Option<f64>) -> bool {
        let second = seconds
            .filter(|value| value.is_finite() && *value >= 0.0 && *value < u64::MAX as f64)
            .map(|value| value.ceil() as u64);
        if self.last_eta == Some(second) {
            return false;
        }
        self.last_eta = Some(second);
        true
    }
}

/// FFmpeg's structured out_time timestamp; unknown/malformed values stay unknown.
pub fn timestamp_seconds(value: &str) -> Option<f64> {
    let mut fields = value.split(':');
    let hours = fields.next()?.parse::<u64>().ok()?;
    let minutes = fields.next()?.parse::<u8>().ok()?;
    let seconds = fields.next()?.parse::<f64>().ok()?;
    if fields.next().is_some()
        || minutes >= 60
        || !seconds.is_finite()
        || !(0.0..60.0).contains(&seconds)
    {
        return None;
    }
    let total = hours as f64 * 3600.0 + f64::from(minutes) * 60.0 + seconds;
    total.is_finite().then_some(total)
}

pub fn thread_budget(logical_cpus: usize) -> usize {
    logical_cpus.saturating_sub(2).max(1)
}

pub fn speed_factor(value: &str) -> Option<f64> {
    let value = value.trim();
    value
        .strip_suffix('x')
        .unwrap_or(value)
        .parse::<f64>()
        .ok()
        .filter(|speed| speed.is_finite() && *speed > 0.0)
}

pub fn eta_seconds(
    position: Option<f64>,
    duration: Option<f64>,
    speed: Option<f64>,
) -> Option<f64> {
    let (position, duration, speed) = (position?, duration?, speed?);
    if !position.is_finite()
        || position < 0.0
        || !duration.is_finite()
        || duration <= 0.0
        || !speed.is_finite()
        || speed <= 0.0
    {
        return None;
    }
    let eta = (duration - position).max(0.0) / speed;
    eta.is_finite().then_some(eta)
}

pub fn format_eta(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0 && *seconds < u64::MAX as f64)
    else {
        return "--".into();
    };
    let seconds = seconds.ceil() as u64;
    if seconds >= 3600 {
        format!(
            "{:02}h {:02}m {:02}s",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}m {:02}s", seconds / 60, seconds % 60)
    }
}

pub fn stderr_duration(line: &str) -> Option<f64> {
    let (_, value) = line.split_once("Duration:")?;
    timestamp_seconds(value.split(',').next()?.trim()).filter(|duration| *duration > 0.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Metrics {
    pub frame: Option<u64>,
    pub position: Option<f64>,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
}

pub(crate) async fn read(
    reader: impl tokio::io::AsyncRead + Unpin,
    mut on_progress: impl FnMut(Metrics),
) -> Result<(), String> {
    let mut metrics = Metrics::default();
    crate::pipe_lines::read(reader, 256, |line, truncated| {
        if truncated {
            return;
        }
        let Some((key, value)) = line.split_once('=') else {
            return;
        };
        let value = value.trim();
        let number = || {
            value
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        match key {
            "frame" => metrics.frame = value.parse().ok(),
            "fps" => metrics.fps = number(),
            "out_time_us" => metrics.position = number().map(|time| time / 1_000_000.0),
            "out_time" if metrics.position.is_none() => metrics.position = timestamp_seconds(value),
            "speed" => metrics.speed = speed_factor(value),
            "progress" if matches!(value, "continue" | "end") => {
                on_progress(metrics);
                metrics = Metrics::default();
            }
            _ => {}
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metrics_refresh_when_duration_is_unknown_without_event_spam() {
        let mut gate = ProgressGate::default();
        let now = std::time::Instant::now();
        assert!(gate.metrics_at(Some(24.0), Some("1x"), now));
        assert!(!gate.metrics_at(
            Some(48.0),
            Some("2x"),
            now + std::time::Duration::from_millis(250)
        ));
        assert!(gate.metrics_at(
            Some(48.0),
            Some("2x"),
            now + std::time::Duration::from_secs(1)
        ));
        assert!(!gate.metrics_at(
            Some(48.0),
            Some("2x"),
            now + std::time::Duration::from_secs(2)
        ));
    }
    #[test]
    fn eta_changes_are_visible_without_duplicate_subsecond_events() {
        let mut gate = ProgressGate::default();
        assert!(gate.should_emit_eta(None));
        assert!(!gate.should_emit_eta(None));
        assert!(gate.should_emit_eta(Some(135.8)));
        assert!(!gate.should_emit_eta(Some(135.2)));
        assert!(gate.should_emit_eta(Some(135.0)));
        assert!(!gate.should_emit_eta(Some(134.9)));
        assert!(gate.should_emit_eta(Some(0.0)));
    }
    #[tokio::test]
    async fn structured_records_reset_unknown_values_and_keep_timestamp_fallback() {
        let input = b"frame=42\nfps= 48.2\nout_time_us=30000000\nout_time=00:00:30.000000\nspeed= 1.85x\nprogress=continue\nframe=90\nfps=NaN\nout_time_us=N/A\nout_time=00:01:00.0\nspeed=N/A\nprogress=end\n";
        let mut records = Vec::new();
        read(input.as_slice(), |metrics| records.push(metrics))
            .await
            .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0],
            Metrics {
                frame: Some(42),
                position: Some(30.0),
                fps: Some(48.2),
                speed: Some(1.85)
            }
        );
        assert_eq!(
            records[1],
            Metrics {
                frame: Some(90),
                position: Some(60.0),
                fps: None,
                speed: None
            }
        );
        assert_eq!(
            eta_seconds(records[0].position, Some(67.0), records[0].speed),
            Some(20.0)
        );
    }

    #[tokio::test]
    async fn oversized_records_do_not_block_later_progress() {
        let mut input = vec![b'x'; 200_000];
        input.extend_from_slice(b"\nfps=30\nprogress=end");
        let mut records = Vec::new();
        read(input.as_slice(), |metrics| records.push(metrics))
            .await
            .unwrap();
        assert_eq!(
            records,
            [Metrics {
                fps: Some(30.0),
                ..Metrics::default()
            }]
        );
    }

    #[test]
    fn duration_speed_and_eta_reject_unknown_values() {
        assert_eq!(
            stderr_duration("Duration: 00:04:30.00, start: 0.00"),
            Some(270.0)
        );
        assert_eq!(stderr_duration("Duration: N/A"), None);
        assert_eq!(stderr_duration("[libass] fontselect: Arial"), None);
        assert_eq!(format_eta(Some(135.0)), "02m 15s");
        assert_eq!(format_eta(Some(3723.1)), "01h 02m 04s");
        assert_eq!(format_eta(None), "--");
        assert_eq!(speed_factor("NaNx"), None);
        assert_eq!(speed_factor("1xxx"), None);
        assert_eq!(eta_seconds(Some(0.0), Some(f64::INFINITY), Some(1.0)), None);
    }
    #[test]
    fn integer_gate_limits_spam_and_unknown_duration_is_not_fabricated() {
        let mut gate = ProgressGate::default();
        assert!(gate.should_emit(None));
        assert!(!gate.should_emit(None));
        assert!(gate.should_emit(Some(0.01)));
        assert!(!gate.should_emit(Some(0.99)));
        assert!(gate.should_emit(Some(1.0)));
        assert!(!gate.should_emit(Some(1.9)));
        assert!(gate.should_emit(Some(99.9)));
        assert!(gate.should_emit(Some(100.0)));
        let mut gate = ProgressGate::default();
        let count = (0..10000)
            .filter(|sample| gate.should_emit(Some(f64::from(*sample) / 100.0)))
            .count();
        assert_eq!(count, 100);
    }
    #[test]
    fn timestamps_and_small_cpu_counts_are_safe() {
        assert_eq!(timestamp_seconds("01:02:03.456"), Some(3723.456));
        for value in [
            "N/A",
            "NaN",
            "00:60:00",
            "00:00:NaN",
            "00:00:01:02",
            "-1:00:00",
        ] {
            assert_eq!(timestamp_seconds(value), None);
        }
        for (cpus, expected) in [(0, 1), (1, 1), (2, 1), (3, 1), (4, 2), (16, 14)] {
            assert_eq!(thread_budget(cpus), expected);
        }
    }
}
