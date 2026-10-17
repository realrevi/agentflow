//! Workspace detection for template questions (read-only, local; nothing leaves the
//! machine). Each detector key fills one kind of answer; the wizard marks those answers
//! "detected" and the user can change them. Reads are capped: at most `MAX_FILES` files are
//! listed and `MAX_STYLE_FILES` style files read, each up to `MAX_READ` bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use super::detect_style::{breakpoints, colors, fonts, spacing};
use super::targets::Target;
use crate::fs::walker;

pub const DETECTORS: [&str; 9] = [
    "languages",
    "packageManager",
    "testFramework",
    "testCommand",
    "uiFramework",
    "colors",
    "fonts",
    "spacing",
    "breakpoints",
];

const MAX_FILES: usize = 5_000;
const MAX_DEPTH: usize = 8;
pub(super) const MAX_STYLE_FILES: usize = 60;
pub(super) const MAX_READ: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingFile {
    pub target: Target,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    /// Detector key -> suggested answer text.
    pub values: BTreeMap<String, String>,
    /// Agent files of each tool already in the workspace.
    pub existing: Vec<ExistingFile>,
}

/// Small text files (manifests, configs); `None` when missing or too large.
pub(super) fn read_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_READ {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

pub(super) fn re(pattern: &str) -> Option<Regex> {
    Regex::new(pattern).ok()
}

fn language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "ts" | "tsx" | "mts" | "cts" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "rs" => "Rust",
        "py" => "Python",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "cs" => "C#",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "dart" => "Dart",
        "css" | "scss" | "sass" | "less" => "CSS",
        "html" | "htm" => "HTML",
        "vue" => "Vue",
        "svelte" => "Svelte",
        _ => return None,
    })
}

/// Relative paths of the workspace files (`.gitignore` respected, hidden files skipped).
fn list(root: &Path) -> Vec<String> {
    walker(root, false)
        .max_depth(Some(MAX_DEPTH))
        .build()
        .flatten()
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|entry| crate::fs::relative_to(root, entry.path()))
        .take(MAX_FILES)
        .collect()
}

pub fn detect(root: &Path) -> Detection {
    let files = list(root);
    let mut values = BTreeMap::new();
    let mut put = |key: &str, value: Option<String>| {
        if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
            values.insert(key.to_owned(), value);
        }
    };
    put("languages", languages(&files));
    let package = read_small(&root.join("package.json"))
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let deps = dependencies(package.as_ref());
    let manager = package_manager(&files, package.as_ref());
    put("packageManager", manager.clone());
    let (framework, command) = tests(root, &files, &deps, package.as_ref(), manager.as_deref());
    put("testFramework", framework);
    put("testCommand", command);
    put("uiFramework", ui_framework(&deps, &files));
    let styles: Vec<&String> = files
        .iter()
        .filter(|f| {
            let lower = f.to_ascii_lowercase();
            [".css", ".scss", ".sass", ".less"]
                .iter()
                .any(|e| lower.ends_with(e))
        })
        .take(MAX_STYLE_FILES)
        .collect();
    let tailwind = files
        .iter()
        .find(|f| {
            f.split('/')
                .next_back()
                .is_some_and(|n| n.starts_with("tailwind.config."))
        })
        .and_then(|f| read_small(&root.join(f)));
    let html: Vec<&String> = files
        .iter()
        .filter(|f| f.ends_with(".html"))
        .take(10)
        .collect();
    put("colors", colors(root, &styles, tailwind.as_deref()));
    put("fonts", fonts(root, &styles, &html, tailwind.as_deref()));
    put(
        "spacing",
        spacing(
            root,
            &styles,
            tailwind.is_some() || deps.contains("tailwindcss"),
        ),
    );
    put(
        "breakpoints",
        breakpoints(
            root,
            &styles,
            tailwind.is_some() || deps.contains("tailwindcss"),
        ),
    );
    Detection {
        values,
        existing: existing(root),
    }
}

fn languages(files: &[String]) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for file in files {
        let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        if let Some(name) = ext.as_deref().and_then(language) {
            *counts.entry(name).or_default() += 1;
        }
    }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let top: Vec<&str> = ranked.into_iter().take(4).map(|(name, _)| name).collect();
    (!top.is_empty()).then(|| top.join(", "))
}

fn dependencies(package: Option<&Value>) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    for key in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(map) = package.and_then(|p| p.get(key)).and_then(Value::as_object) {
            deps.extend(map.keys().cloned());
        }
    }
    deps
}

fn has(files: &[String], name: &str) -> bool {
    files.iter().any(|f| f == name)
}

fn package_manager(files: &[String], package: Option<&Value>) -> Option<String> {
    if let Some(declared) = package
        .and_then(|p| p.get("packageManager"))
        .and_then(Value::as_str)
    {
        return declared.split('@').next().map(str::to_owned);
    }
    let pick = [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
        ("package-lock.json", "npm"),
        ("Cargo.toml", "cargo"),
        ("poetry.lock", "poetry"),
        ("uv.lock", "uv"),
        ("pyproject.toml", "pip"),
        ("requirements.txt", "pip"),
        ("go.mod", "go"),
    ];
    if let Some((_, name)) = pick.iter().find(|(file, _)| has(files, file)) {
        return Some((*name).to_owned());
    }
    if package.is_some() {
        return Some("npm".into());
    }
    let dotnet = files
        .iter()
        .any(|f| f.ends_with(".csproj") || f.ends_with(".sln"));
    dotnet.then(|| "dotnet".into())
}

fn tests(
    root: &Path,
    files: &[String],
    deps: &BTreeSet<String>,
    package: Option<&Value>,
    manager: Option<&str>,
) -> (Option<String>, Option<String>) {
    let js = [
        ("vitest", "Vitest"),
        ("jest", "Jest"),
        ("mocha", "Mocha"),
        ("@playwright/test", "Playwright"),
        ("cypress", "Cypress"),
        ("ava", "AVA"),
    ];
    let found: Vec<&str> = js
        .iter()
        .filter(|(dep, _)| deps.contains(*dep))
        .map(|(_, name)| *name)
        .collect();
    if !found.is_empty() || package.is_some() {
        let script = package
            .and_then(|p| p.pointer("/scripts/test"))
            .and_then(Value::as_str)
            .is_some();
        let pm = manager
            .filter(|m| ["pnpm", "yarn", "bun", "npm"].contains(m))
            .unwrap_or("npm");
        let command = script.then(|| format!("{pm} test"));
        return ((!found.is_empty()).then(|| found.join(", ")), command);
    }
    if has(files, "Cargo.toml") {
        return (Some("cargo test".into()), Some("cargo test".into()));
    }
    let python = [
        "pyproject.toml",
        "setup.cfg",
        "requirements.txt",
        "pytest.ini",
    ]
    .iter()
    .filter_map(|f| read_small(&root.join(f)))
    .any(|text| text.contains("pytest"));
    if python || has(files, "pytest.ini") {
        return (Some("pytest".into()), Some("pytest".into()));
    }
    if has(files, "go.mod") {
        return (Some("go test".into()), Some("go test ./...".into()));
    }
    let projects: Vec<String> = files
        .iter()
        .filter(|f| f.ends_with(".csproj"))
        .take(20)
        .filter_map(|f| read_small(&root.join(f)))
        .collect();
    let dotnet = [("xunit", "xUnit"), ("nunit", "NUnit"), ("MSTest", "MSTest")];
    let found: Vec<&str> = dotnet
        .iter()
        .filter(|(needle, _)| {
            projects
                .iter()
                .any(|p| p.to_lowercase().contains(&needle.to_lowercase()))
        })
        .map(|(_, name)| *name)
        .collect();
    if !projects.is_empty() {
        return (
            (!found.is_empty()).then(|| found.join(", ")),
            Some("dotnet test".into()),
        );
    }
    (None, None)
}

fn ui_framework(deps: &BTreeSet<String>, files: &[String]) -> Option<String> {
    let known = [
        ("next", "Next.js (React)"),
        ("nuxt", "Nuxt (Vue)"),
        ("@sveltejs/kit", "SvelteKit"),
        ("astro", "Astro"),
        ("@angular/core", "Angular"),
        ("solid-js", "SolidJS"),
        ("svelte", "Svelte"),
        ("vue", "Vue"),
        ("preact", "Preact"),
        ("react", "React"),
    ];
    let mut name = known
        .iter()
        .find(|(dep, _)| deps.contains(*dep))
        .map(|(_, name)| (*name).to_owned());
    if name.is_none() && files.iter().any(|f| f.ends_with(".html")) {
        name = Some("HTML + CSS".into());
    }
    if deps.contains("tailwindcss") {
        name = Some(match name {
            Some(base) => format!("{base} + Tailwind CSS"),
            None => "Tailwind CSS".into(),
        });
    }
    name
}

/// Agent files per tool that already exist (the wizard warns before touching them).
fn existing(root: &Path) -> Vec<ExistingFile> {
    let mut out = Vec::new();
    let mut file = |target: Target, path: &str| {
        if root.join(path).is_file() {
            out.push(ExistingFile {
                target,
                path: path.to_owned(),
            });
        }
    };
    file(Target::ClaudeCode, "CLAUDE.md");
    file(Target::Codex, "AGENTS.md");
    file(Target::Gemini, "GEMINI.md");
    let dirs = [
        (Target::ClaudeCode, ".claude/agents", ".md"),
        (Target::Codex, ".codex/agents", ".toml"),
        (Target::Gemini, ".gemini/agents", ".md"),
        (Target::Cursor, ".cursor/rules", ".mdc"),
        (Target::Agentflow, ".agentflow/agents", ".md"),
    ];
    for (target, dir, ext) in dirs {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.ends_with(ext))
            .collect();
        names.sort();
        out.extend(names.into_iter().take(50).map(|name| ExistingFile {
            target,
            path: format!("{dir}/{name}"),
        }));
    }
    out
}
