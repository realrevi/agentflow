//! Panics are written to `crash.log` in the app data folder (a GUI app has no visible
//! stderr), then the default hook runs as before. Release builds unwind (Cargo.toml), so a
//! panic in one worker thread or command does not take the whole app down.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = "crash.log";
/// Older entries are dropped once the log grows past this.
const MAX_BYTES: u64 = 256 * 1024;

pub fn install_hook(data_dir: &Path) {
    let path = data_dir.join(FILE);
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let entry = entry(thread.name().unwrap_or("unnamed"), &info.to_string());
        let _ = append(&path, &entry);
        default_hook(info);
    }));
}

/// One log entry: when, which version and thread, and the panic message with its location.
fn entry(thread: &str, message: &str) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format!(
        "[unix {secs}] AgentFlow {} ({} {}), thread '{thread}': {message}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn append(path: &PathBuf, entry: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let too_big = std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES);
    let mut file = OpenOptions::new()
        .create(true)
        .append(!too_big)
        .write(true)
        .truncate(too_big)
        .open(path)?;
    file.write_all(entry.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn entries_are_appended_and_the_log_stays_small() {
        let dir = TestDir::new("crash-log");
        let path = dir.path().join(FILE);
        let first = entry("main", "boom at src/x.rs:1:1");
        assert!(first.contains("thread 'main': boom at src/x.rs:1:1"));
        assert!(first.contains(env!("CARGO_PKG_VERSION")));
        append(&path, &first).expect("append");
        append(&path, &entry("worker", "second")).expect("append");
        let text = std::fs::read_to_string(&path).expect("read");
        assert_eq!(text.lines().count(), 2);

        std::fs::write(&path, vec![b'x'; (MAX_BYTES + 1) as usize]).expect("fill");
        append(&path, &entry("main", "fresh")).expect("append");
        let text = std::fs::read_to_string(&path).expect("read");
        assert_eq!(text.lines().count(), 1, "an oversized log starts over");
    }
}
