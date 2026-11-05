use std::collections::VecDeque;
use tokio::io::AsyncRead;

/// Drain stderr completely without allocating an unbounded String for one hostile line.
pub async fn monitor(
    reader: impl AsyncRead + Unpin,
    mut on_line: impl FnMut(&str),
) -> Result<String, String> {
    let mut tail = VecDeque::with_capacity(100);
    crate::pipe_lines::read(reader, 2048, |line, truncated| {
        if tail.len() == 100 {
            tail.pop_front();
        }
        if !truncated {
            on_line(line);
        }
        let mut text = line.to_owned();
        if truncated {
            text.push_str(" [truncated]");
        }
        tail.push_back(text);
    })
    .await?;
    Ok(tail.into_iter().collect::<Vec<_>>().join("\n"))
}

#[cfg(test)]
async fn tail(reader: impl AsyncRead + Unpin) -> Result<String, String> {
    monitor(reader, |_| {}).await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn duration_fallback_ignores_diagnostics() {
        let mut duration = None;
        let input = b"[info] Duration: 00:03:00.00, bitrate: 123\r\n[libass] fontselect: Arial\n";
        let tail = super::monitor(input.as_slice(), |line| {
            if let Some(value) = crate::progress::stderr_duration(line) {
                duration = Some(value);
            }
        })
        .await
        .expect("monitor");
        assert_eq!(duration, Some(180.0));
        assert_eq!(tail.lines().count(), 2);
    }
    #[tokio::test]
    async fn last_hundred_normal_lines_are_preserved_exactly() {
        let input = (1..=105)
            .map(|number| format!("diagnostic {number}\n"))
            .collect::<String>();
        let actual = super::tail(input.as_bytes()).await.expect("tail");
        let expected = (6..=105)
            .map(|number| format!("diagnostic {number}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actual, expected);
    }
    #[tokio::test]
    async fn diagnostics_cap_long_lines_and_preserve_final_errors() {
        let mut bytes = vec![b'x'; 200_000];
        bytes.extend_from_slice(b"\nSubtitle parser failed\n");
        let text = super::tail(bytes.as_slice()).await.expect("drain");
        assert!(text.len() < 2200);
        assert!(text.contains("[truncated]") && text.contains("Subtitle parser failed"));
    }
}
