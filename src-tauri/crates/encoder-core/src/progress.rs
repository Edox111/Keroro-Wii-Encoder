/// Timing updates require a changed integer percentage or ETA second.
#[derive(Default)]
pub struct ProgressGate {
    last: Option<Option<u8>>,
    last_eta: Option<Option<u64>>,
}
impl ProgressGate {
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

#[derive(Clone, Default)]
pub struct StderrProgress {
    pub duration: Option<f64>,
    pub position: Option<f64>,
    pub speed: Option<f64>,
}
impl StderrProgress {
    pub fn update(&mut self, line: &str) -> bool {
        let mut changed = false;
        if let Some((_, value)) = line.split_once("Duration:") {
            self.duration = value
                .split(',')
                .next()
                .and_then(|value| timestamp_seconds(value.trim()))
                .filter(|duration| *duration > 0.0);
            changed = true;
        }
        if let Some((_, value)) = line.split_once("time=") {
            self.position = value.split_whitespace().next().and_then(timestamp_seconds);
            changed = true;
        }
        if let Some((_, value)) = line.split_once("speed=") {
            self.speed = value.split_whitespace().next().and_then(speed_factor);
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn stderr_duration_time_speed_and_eta_are_parsed_without_inventing_values() {
        let mut progress = StderrProgress::default();
        assert!(progress.update("[info] Duration: 00:04:30.00, start: 0.000, bitrate: 123"));
        assert!(progress.update("frame= 42 fps=30 time=00:02:15.00 bitrate=123kbits/s speed=1.0x"));
        assert_eq!(
            eta_seconds(progress.position, progress.duration, progress.speed),
            Some(135.0)
        );
        assert_eq!(format_eta(Some(135.0)), "02m 15s");
        assert_eq!(eta_seconds(Some(30.0), Some(90.0), Some(2.0)), Some(30.0));
        assert_eq!(format_eta(Some(3723.1)), "01h 02m 04s");
        progress.update("time=N/A speed=0x");
        assert!(eta_seconds(progress.position, progress.duration, progress.speed).is_none());
        assert_eq!(format_eta(None), "--");
        assert_eq!(speed_factor("NaNx"), None);
        assert_eq!(speed_factor("1xxx"), None);
        for noise in [
            "[warning] A subtitle font was substituted",
            "[libass] fontselect: Arial",
            "[error] recoverable decoder diagnostic",
        ] {
            assert!(!progress.update(noise));
        }
        assert_eq!(progress.duration, Some(270.0));
        assert_eq!(progress.position, None);
        assert_eq!(progress.speed, None);
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
