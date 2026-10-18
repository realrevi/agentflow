//! Relative paths that come from outside (skill files of a library repository): only plain
//! `a/b/c` paths inside their folder are accepted. No absolute paths, drive letters, `..`,
//! empty or dot segments, control characters or names Windows cannot create (zip-slip and
//! friends). The workspace sandbox checks again when writing.

const MAX_PATH: usize = 240;
const MAX_DEPTH: usize = 8;
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// The path with `/` separators, or why it is refused.
pub fn safe_relative(path: &str) -> Result<String, String> {
    let unified = path.replace('\\', "/");
    if unified.is_empty() || unified.len() > MAX_PATH {
        return Err(format!("{path}: empty or too long"));
    }
    if unified.starts_with('/') || unified.contains(':') {
        return Err(format!("{path}: absolute paths are not allowed"));
    }
    let parts: Vec<&str> = unified.split('/').collect();
    if parts.len() > MAX_DEPTH {
        return Err(format!("{path}: nested too deep"));
    }
    for part in &parts {
        let lower = part.to_ascii_lowercase();
        let stem = lower.split('.').next().unwrap_or_default();
        let bad = part.is_empty()
            || *part == "."
            || *part == ".."
            || part.ends_with('.')
            || part.ends_with(' ')
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
            || RESERVED.contains(&stem);
        if bad {
            return Err(format!("{path}: \"{part}\" is not a safe name"));
        }
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::safe_relative;

    #[test]
    fn plain_relative_paths_pass() {
        assert_eq!(safe_relative("scripts/run.py"), Ok("scripts/run.py".into()));
        assert_eq!(safe_relative(r"docs\guide.md"), Ok("docs/guide.md".into()));
        assert_eq!(
            safe_relative(".hidden/config.json"),
            Ok(".hidden/config.json".into())
        );
    }

    #[test]
    fn escapes_and_absolute_paths_are_refused() {
        for bad in [
            "../etc/passwd",
            "a/../../b",
            "/etc/passwd",
            r"C:\Windows\system.ini",
            r"\\server\share\x",
            "a//b",
            "./a",
            "a/.",
            "CON.txt",
            "a/nul",
            "trailing. ",
            "bad|name",
            "x\u{0}y",
            "",
        ] {
            assert!(safe_relative(bad).is_err(), "{bad:?} should be refused");
        }
        let deep = ["d"; 9].join("/");
        assert!(safe_relative(&deep).is_err());
    }
}
