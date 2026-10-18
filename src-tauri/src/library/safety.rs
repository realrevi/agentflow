//! The library's safety scan: warnings (never a block) for content that downloads and runs
//! code, turns off permissions or tells the agent to ignore the user, sends data to an
//! external URL, hides long encoded blobs, or ships executable scripts. Every pattern is
//! here, each with tests; a new flag id needs a `library.flag.<id>` text in every locale.

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Excerpts per flag and file; the scan reports where, the preview shows everything.
const MAX_HITS: usize = 3;
const EXCERPT_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flag {
    /// `downloadExec`, `disablePermissions`, `ignoreUser`, `sendsData`, `encodedBlob` or
    /// `executable`.
    pub id: String,
    /// The file it was found in (relative to the item; empty = the main file).
    pub file: String,
    /// What matched (a line excerpt, or the script's name).
    pub excerpt: String,
}

/// (flag id, case-insensitive pattern).
const PATTERNS: &[(&str, &str)] = &[
    // curl/wget piped into a shell or interpreter; `sh -c "$(curl ...)"`; `bash <(curl ...)`.
    (
        "downloadExec",
        r"\b(curl|wget)\b[^\n|]*\|\s*(sudo\s+)?(ba|z|da|k|fi)?sh\b",
    ),
    (
        "downloadExec",
        r"\b(curl|wget)\b[^\n|]*\|\s*(sudo\s+)?(python[0-9.]*|node|perl|ruby|php)\b",
    ),
    (
        "downloadExec",
        r"\b(ba|z)?sh\s+(-c\s+)?[\x22']?\$\(\s*(curl|wget)\b",
    ),
    ("downloadExec", r"\b(ba|z)?sh\s+<\(\s*(curl|wget)\b"),
    // PowerShell: iwr/irm/Invoke-WebRequest piped into iex; DownloadString into iex.
    (
        "downloadExec",
        r"\b(iwr|irm|invoke-webrequest|invoke-restmethod)\b[^\n|]*\|\s*(iex|invoke-expression)\b",
    ),
    (
        "downloadExec",
        r"\b(iex|invoke-expression)\b[^\n]*\b(downloadstring|iwr|irm|invoke-webrequest|invoke-restmethod)\b",
    ),
    // Turning off the tools' approvals or sandboxes.
    (
        "disablePermissions",
        r"--dangerously-skip-permissions|\bbypasspermissions\b|--yolo\b|--dangerously-bypass-approvals-and-sandbox|\bdanger-full-access\b",
    ),
    (
        "disablePermissions",
        r#"\b(approval_policy|ask[-_]for[-_]approval)\b\s*[=: ]\s*"?never"#,
    ),
    (
        "disablePermissions",
        r"\b(disable|turn off|skip|bypass)\s+(all\s+|the\s+)?(permission|approval|confirmation|sandbox)",
    ),
    (
        "disablePermissions",
        r"\b(never|do not|don't)\s+ask\s+(the user\s+)?(for\s+)?(permission|approval)",
    ),
    (
        "disablePermissions",
        r"\b(never|do not|don't)\s+(ask|wait)\s+(the user\s+)?(for\s+)?confirmation\s+(before|to)\s",
    ),
    ("disablePermissions", r"\bauto-?approve\s+(all|every|any)\b"),
    // Overriding the user or the system instructions, or hiding things from the user.
    (
        "ignoreUser",
        r"\bignore\s+(all\s+|any\s+)?(previous|prior|earlier|above|system)\s+(instructions|prompts?|rules)",
    ),
    (
        "ignoreUser",
        r"\b(ignore|disregard|override)\s+(what\s+)?the\s+user('s)?\b",
    ),
    (
        "ignoreUser",
        r"\b(do not|don't|never)\s+(tell|inform|show|mention\s+(it\s+)?to)\s+the\s+user\b",
    ),
    (
        "ignoreUser",
        r"\b(without|before)\s+(telling|informing|asking)\s+the\s+user\b|\bhide\s+(this|it|them)\s+from\s+the\s+user\b",
    ),
    // Sending data out: uploads to a URL, posting to webhooks or paste sites.
    (
        "sendsData",
        r"\bcurl\b[^\n]*\s(?-i:-d|--data(-binary|-raw|-urlencode)?|-F|--form|-T|--upload-file)\s[^\n]*https?://",
    ),
    (
        "sendsData",
        r"\bcurl\b[^\n]*https?://[^\n]*\s(?-i:-d|--data(-binary|-raw|-urlencode)?|-F|--form|-T|--upload-file)\s",
    ),
    (
        "sendsData",
        r"\b(requests|httpx|axios)\.(post|put)\s*\(\s*[\x22'`]https?://",
    ),
    (
        "sendsData",
        r"\b(invoke-webrequest|invoke-restmethod|iwr|irm)\b[^\n]*-method\s+(post|put)\b",
    ),
    (
        "sendsData",
        r"\b(send|upload|post|exfiltrate|forward)\s+(it|them|this|that|the\s+\w+|all\s+\w+|\w+)?\s*(\w+\s+){0,4}to\s+https?://",
    ),
    (
        "sendsData",
        r"hooks\.slack\.com/|discord(app)?\.com/api/webhooks|webhook\.site|requestbin|pipedream\.net|ngrok\.(io|app)|pastebin\.com|transfer\.sh",
    ),
];

/// A run of base64 (or hex) this long with no spaces is an encoded blob.
const BLOB: &str = r"[A-Za-z0-9+/]{200,}={0,2}|[0-9a-fA-F]{200,}";

/// File names in a skill that are programs or scripts (copied, never run by AgentFlow).
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "psm1",
    "bat",
    "cmd",
    "py",
    "js",
    "mjs",
    "cjs",
    "ts",
    "rb",
    "pl",
    "php",
    "lua",
    "exe",
    "dll",
    "so",
    "dylib",
    "jar",
    "bin",
    "msi",
    "com",
    "scr",
    "vbs",
    "wsf",
    "applescript",
    "command",
    "appimage",
];

fn patterns() -> &'static [(&'static str, Regex)] {
    static COMPILED: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        PATTERNS
            .iter()
            .filter_map(|(id, pattern)| {
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .ok()
                    .map(|re| (*id, re))
            })
            .collect()
    })
}

fn blob() -> Option<&'static Regex> {
    static COMPILED: OnceLock<Option<Regex>> = OnceLock::new();
    COMPILED.get_or_init(|| Regex::new(BLOB).ok()).as_ref()
}

fn excerpt(text: &str, start: usize, end: usize) -> String {
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let line = text[line_start..line_end].trim();
    if line.chars().count() <= EXCERPT_CHARS {
        return line.to_owned();
    }
    // Long lines: the match and a little around it.
    let from = text[..start]
        .char_indices()
        .rev()
        .nth(20)
        .map_or(line_start, |(i, _)| i.max(line_start));
    let cut: String = text[from..line_end].chars().take(EXCERPT_CHARS).collect();
    format!("…{}…", cut.trim())
}

/// Flags in one text file (`file` relative to the item, empty for the main file).
pub fn scan_text(file: &str, text: &str) -> Vec<Flag> {
    let mut flags: Vec<Flag> = Vec::new();
    let mut push = |id: &str, excerpt: String| {
        let same = flags.iter().filter(|f| f.id == id).count();
        if same < MAX_HITS && !flags.iter().any(|f| f.id == id && f.excerpt == excerpt) {
            flags.push(Flag {
                id: id.to_owned(),
                file: file.to_owned(),
                excerpt,
            });
        }
    };
    for (id, re) in patterns() {
        for m in re.find_iter(text).take(MAX_HITS) {
            let line = excerpt(text, m.start(), m.end());
            if *id == "sendsData" && is_placeholder_host(&line) {
                continue;
            }
            push(id, line);
        }
    }
    if let Some(re) = blob() {
        for m in re.find_iter(text).take(MAX_HITS) {
            let head: String = m.as_str().chars().take(24).collect();
            push(
                "encodedBlob",
                format!("{head}… ({} characters)", m.as_str().len()),
            );
        }
    }
    flags
}

/// Code samples posting to `example.com` or the local machine send nothing anywhere.
fn is_placeholder_host(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "example.com",
        "example.org",
        "example.net",
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
    ]
    .iter()
    .any(|host| lower.contains(host))
}

pub fn is_executable(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let in_scripts = path
        .split('/')
        .any(|part| part == "scripts" || part == "bin");
    let ext = name.rsplit_once('.').map(|(_, ext)| ext);
    ext.is_some_and(|ext| EXECUTABLE_EXTENSIONS.contains(&ext)) || (in_scripts && ext.is_none())
}

/// One `executable` flag listing a skill's scripts and programs.
pub fn scan_files<'a>(paths: impl Iterator<Item = &'a str>) -> Option<Flag> {
    let names: Vec<&str> = paths.filter(|p| is_executable(p)).collect();
    (!names.is_empty()).then(|| Flag {
        id: "executable".into(),
        file: String::new(),
        excerpt: names.join(", "),
    })
}
