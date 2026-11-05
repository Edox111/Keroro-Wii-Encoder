use tokio::io::{AsyncRead, AsyncReadExt};

/// Reuse bounded storage while draining even oversized or non-UTF-8 records.
pub(crate) async fn read(
    mut reader: impl AsyncRead + Unpin,
    limit: usize,
    mut on_line: impl FnMut(&str, bool),
) -> Result<(), String> {
    let mut line = Vec::with_capacity(limit);
    let mut chunk = [0u8; 8192];
    let mut truncated = false;
    let mut previous_cr = false;
    loop {
        let count = reader.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        for &byte in &chunk[..count] {
            if byte == b'\n' || byte == b'\r' {
                if byte == b'\n' && previous_cr {
                    previous_cr = false;
                    continue;
                }
                previous_cr = byte == b'\r';
                on_line(&String::from_utf8_lossy(&line), truncated);
                line.clear();
                truncated = false;
            } else {
                previous_cr = false;
                if line.len() < limit {
                    line.push(byte);
                } else {
                    truncated = true;
                }
            }
        }
    }
    if !line.is_empty() || truncated {
        on_line(&String::from_utf8_lossy(&line), truncated);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn split_records_crlf_and_oversized_lines_remain_bounded() {
        let (mut writer, reader) = tokio::io::duplex(3);
        let task = tokio::spawn(async move {
            writer.write_all(b"fps=24\r\n").await.unwrap();
            writer.write_all(&[b'x'; 100_000]).await.unwrap();
            writer.write_all(b"\rspeed=2x\nend").await.unwrap();
        });
        let mut lines = Vec::new();
        super::read(reader, 16, |line, truncated| {
            lines.push((line.to_owned(), truncated))
        })
        .await
        .unwrap();
        task.await.unwrap();
        assert_eq!(
            lines,
            [
                ("fps=24".into(), false),
                ("x".repeat(16), true),
                ("speed=2x".into(), false),
                ("end".into(), false)
            ]
        );
    }
}
