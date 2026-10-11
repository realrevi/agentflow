//! Names for per-workspace data in the app data dir, and the current time.

use std::time::{SystemTime, UNIX_EPOCH};

/// `<folder>-<hash>` for a workspace (readable and collision-safe), `_home` without one.
pub fn workspace_key(workspace: Option<&str>) -> String {
    let Some(path) = workspace else {
        return "_home".into();
    };
    let folder: String = path
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(40)
        .collect();
    format!("{folder}-{:016x}", fnv1a(path.as_bytes()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Milliseconds since the Unix epoch (0 if the clock is before it).
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_stable_readable_and_distinct() {
        let ws = Some("/home/me/projects/demo-app");
        assert_eq!(workspace_key(ws), workspace_key(ws));
        assert!(workspace_key(ws).starts_with("demo-app-"));
        assert_ne!(workspace_key(ws), workspace_key(Some("/other/demo-app")));
        assert_eq!(workspace_key(None), "_home");
        assert!(workspace_key(Some("C:\\Users\\me\\x y")).starts_with("xy-"));
        assert!(now_ms() > 1_700_000_000_000);
    }
}
