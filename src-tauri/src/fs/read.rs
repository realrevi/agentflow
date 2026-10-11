//! Reading files for the editor: size limit, binary detection, UTF-8 with a lossy fallback.

use std::fs::{File, Metadata};
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::model::FileContent;
use crate::error::{AppError, AppResult};

pub const MAX_READ_BYTES: u64 = 5 * 1024 * 1024;
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

pub fn read_file(path: &Path) -> AppResult<FileContent> {
    let file = File::open(path)?;
    // Metadata before reading: if the file changes in between, the next save reports a
    // conflict instead of silently overwriting the newer version.
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(AppError::InvalidInput(format!(
            "not a file: {}",
            path.display()
        )));
    }
    if meta.len() > MAX_READ_BYTES {
        return Err(too_large(meta.len()));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    // `take` guards against a file that grew after the size check.
    file.take(MAX_READ_BYTES + 1).read_to_end(&mut bytes)?;
    let size = bytes.len() as u64;
    if size > MAX_READ_BYTES {
        return Err(too_large(size));
    }

    let mtime = mtime_ms(&meta);
    let sniff = &bytes[..bytes.len().min(BINARY_SNIFF_BYTES)];
    if sniff.contains(&0) {
        return Ok(FileContent {
            content: None,
            binary: true,
            lossy: false,
            size,
            mtime,
        });
    }
    let (content, lossy) = match String::from_utf8(bytes) {
        Ok(text) => (text, false),
        Err(err) => (String::from_utf8_lossy(err.as_bytes()).into_owned(), true),
    };
    Ok(FileContent {
        content: Some(content),
        binary: false,
        lossy,
        size,
        mtime,
    })
}

/// Milliseconds since the Unix epoch (0 if the platform cannot tell).
pub fn mtime_ms(meta: &Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

fn too_large(size: u64) -> AppError {
    AppError::TooLarge {
        size,
        limit: MAX_READ_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;
    use std::fs;

    #[test]
    fn reads_utf8_text() {
        let dir = TestDir::new("fs-read");
        let path = dir.path().join("hello.md");
        fs::write(&path, "héllo\n").expect("write");
        let read = read_file(&path).expect("read");
        assert_eq!(read.content.as_deref(), Some("héllo\n"));
        assert!(!read.binary && !read.lossy);
        assert_eq!(read.size, 7);
        assert!(read.mtime > 0);
    }

    #[test]
    fn detects_binary_files() {
        let dir = TestDir::new("fs-read");
        let path = dir.path().join("image.bin");
        fs::write(&path, [0x89, b'P', b'N', b'G', 0, 1, 2]).expect("write");
        let read = read_file(&path).expect("read");
        assert!(read.binary);
        assert_eq!(read.content, None);

        // A NUL byte after the first 8 KB does not make the file binary.
        let late = dir.path().join("late.txt");
        let mut bytes = vec![b'a'; BINARY_SNIFF_BYTES];
        bytes.push(0);
        fs::write(&late, bytes).expect("write");
        assert!(!read_file(&late).expect("read").binary);
    }

    #[test]
    fn invalid_utf8_is_decoded_lossily() {
        let dir = TestDir::new("fs-read");
        let path = dir.path().join("latin1.txt");
        fs::write(&path, b"caf\xe9").expect("write");
        let read = read_file(&path).expect("read");
        assert!(read.lossy);
        assert_eq!(read.content.as_deref(), Some("caf\u{fffd}"));
    }

    #[test]
    fn files_over_the_limit_are_rejected_with_their_size() {
        let dir = TestDir::new("fs-read");
        let path = dir.path().join("big.log");
        let file = File::create(&path).expect("create");
        file.set_len(MAX_READ_BYTES + 1).expect("grow");
        match read_file(&path) {
            Err(AppError::TooLarge { size, limit }) => {
                assert_eq!(size, MAX_READ_BYTES + 1);
                assert_eq!(limit, MAX_READ_BYTES);
            }
            other => panic!("expected TooLarge, got {other:?}"),
        }
        let exact = dir.path().join("exact.txt");
        fs::write(&exact, vec![b'x'; MAX_READ_BYTES as usize]).expect("write");
        assert!(read_file(&exact).is_ok(), "exactly 5 MB is allowed");
    }
}
