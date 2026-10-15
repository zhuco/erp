use axum::body::{Body, Bytes};
use futures::stream;
use sha2::{Digest, Sha256};
use std::{io, path::Path};
use tokio::io::{AsyncReadExt, Take};

pub(crate) const MAX_EXPORT_FILE_LEN: u64 = 1024 * 1024 * 1024;
const EXPORT_CHUNK_LEN: usize = 64 * 1024;

pub(crate) async fn export_file_body(path: &Path) -> io::Result<(Body, u64)> {
    export_file_body_checked(path, None).await
}

pub(crate) async fn export_file_body_checked(
    path: &Path,
    receipt: Option<(u64, String)>,
) -> io::Result<(Body, u64)> {
    let file = tokio::fs::File::open(path).await?;
    let metadata = file.metadata().await?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "export path is not a regular file",
        ));
    }

    let expected_len = metadata.len();
    if receipt
        .as_ref()
        .is_some_and(|(bytes, _)| *bytes != expected_len)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "export receipt length mismatch",
        ));
    }
    if expected_len > MAX_EXPORT_FILE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "export file exceeds the 1 GiB download limit",
        ));
    }

    let state = ExportStreamState {
        file: file.take(expected_len),
        expected_len,
        bytes_read: 0,
        finished: false,
        hash: receipt.as_ref().map(|_| Sha256::new()),
        digest: receipt.map(|(_, hash)| hash),
    };
    let body_stream = stream::try_unfold(state, |mut state| async move {
        if state.finished {
            return Ok(None);
        }

        let remaining = state.expected_len - state.bytes_read;
        if remaining == 0 {
            let mut extra = [0_u8; 1];
            if state.file.get_mut().read(&mut extra).await? != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "export file grew while it was being downloaded",
                ));
            }
            return Ok(None);
        }

        let chunk_len = usize::try_from(remaining.min(EXPORT_CHUNK_LEN as u64))
            .map_err(|_| io::Error::other("invalid export chunk length"))?;
        let mut chunk = vec![0_u8; chunk_len];
        let count = state.file.read(&mut chunk).await?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "export file was truncated while it was being downloaded",
            ));
        }
        chunk.truncate(count);
        if let Some(hash) = state.hash.as_mut() {
            hash.update(&chunk);
        }
        state.bytes_read += count as u64;
        if state.bytes_read == state.expected_len {
            let mut extra = [0_u8; 1];
            if state.file.get_mut().read(&mut extra).await? != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "export file grew while it was being downloaded",
                ));
            }
            state.finished = true;
            if let Some(hash) = state.hash.take()
                && state.digest.as_deref() != Some(hex::encode(hash.finalize()).as_str())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "export receipt hash mismatch",
                ));
            }
        }
        Ok(Some((Bytes::from(chunk), state)))
    });

    Ok((Body::from_stream(body_stream), expected_len))
}

struct ExportStreamState {
    file: Take<tokio::fs::File>,
    expected_len: u64,
    bytes_read: u64,
    finished: bool,
    hash: Option<Sha256>,
    digest: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use tokio::io::AsyncWriteExt;

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("erp-export-{label}-{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn export_streams_large_file_in_bounded_chunks() -> anyhow::Result<()> {
        let path = test_path("large.csv");
        let expected_len = 8 * 1024 * 1024 + 17;
        let file = tokio::fs::File::create(&path).await?;
        file.set_len(expected_len).await?;

        let (body, content_len) = export_file_body(&path).await?;
        assert_eq!(content_len, expected_len);
        let mut body = std::pin::pin!(body);
        let mut observed_len = 0_u64;
        let mut chunks = 0_usize;
        while let Some(frame) = body.frame().await {
            let frame = frame?;
            if let Some(data) = frame.data_ref() {
                assert!(data.len() <= EXPORT_CHUNK_LEN);
                observed_len += data.len() as u64;
                chunks += 1;
            }
        }
        assert_eq!(observed_len, expected_len);
        assert!(chunks > 1);
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    #[tokio::test]
    async fn export_stream_rejects_oversized_and_non_file_paths() -> anyhow::Result<()> {
        let oversized = test_path("oversized.csv");
        let file = tokio::fs::File::create(&oversized).await?;
        file.set_len(MAX_EXPORT_FILE_LEN + 1).await?;
        let error = export_file_body(&oversized)
            .await
            .err()
            .expect("oversized file must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        tokio::fs::remove_file(oversized).await?;

        let directory = test_path("directory");
        tokio::fs::create_dir(&directory).await?;
        let error = export_file_body(&directory)
            .await
            .err()
            .expect("directory must be rejected");
        assert!(matches!(
            error.kind(),
            io::ErrorKind::InvalidInput | io::ErrorKind::PermissionDenied
        ));
        tokio::fs::remove_dir(directory).await?;
        Ok(())
    }

    #[tokio::test]
    async fn export_stream_reports_truncation_and_growth() -> anyhow::Result<()> {
        let truncated = test_path("truncated.csv");
        tokio::fs::write(&truncated, vec![b'a'; EXPORT_CHUNK_LEN + 1]).await?;
        let (body, _) = export_file_body(&truncated).await?;
        tokio::fs::OpenOptions::new()
            .write(true)
            .open(&truncated)
            .await?
            .set_len(1)
            .await?;
        let error = body.collect().await.expect_err("truncated stream");
        assert_eq!(
            error.to_string(),
            "export file was truncated while it was being downloaded"
        );
        tokio::fs::remove_file(truncated).await?;

        let grown = test_path("grown.csv");
        tokio::fs::write(&grown, b"a").await?;
        let (body, _) = export_file_body(&grown).await?;
        let mut append = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&grown)
            .await?;
        append.write_all(b"b").await?;
        append.flush().await?;
        let error = body.collect().await.expect_err("grown stream");
        assert_eq!(
            error.to_string(),
            "export file grew while it was being downloaded"
        );
        tokio::fs::remove_file(grown).await?;
        Ok(())
    }
}
