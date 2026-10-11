//! Lists built-in and user themes, loads them by id and imports new VS Code themes.
//! User themes live in `<app config dir>/themes/<slug>.json` with id `user:<slug>`.

use std::fs;
use std::path::{Path, PathBuf};

use super::builtin::{self, BUILTIN_THEMES};
use super::model::{ThemeData, ThemeSummary};
use super::parse::{parse_theme, ParsedTheme};
use crate::error::{AppError, AppResult};
use crate::util::write_atomic;

const USER_PREFIX: &str = "user:";
const MAX_IMPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_SLUG_CHARS: usize = 48;

#[derive(Debug, Clone)]
pub struct ThemeStore {
    dir: PathBuf,
}

impl ThemeStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Built-in themes first, then user themes sorted by name. Broken user files are skipped.
    pub fn list(&self) -> Vec<ThemeSummary> {
        let mut user: Vec<ThemeSummary> = self
            .user_files()
            .into_iter()
            .filter_map(|(slug, path)| match read_theme(&path, &slug) {
                Ok(theme) => Some(summary(format!("{USER_PREFIX}{slug}"), &theme, false)),
                Err(err) => {
                    tracing::warn!("skipping theme {}: {err}", path.display());
                    None
                }
            })
            .collect();
        user.sort_by_key(|theme| theme.name.to_lowercase());

        BUILTIN_THEMES
            .iter()
            .filter_map(|theme| {
                parse_theme(theme.source, theme.id)
                    .ok()
                    .map(|parsed| summary(theme.id.into(), &parsed, true))
            })
            .chain(user)
            .collect()
    }

    pub fn get(&self, id: &str) -> AppResult<ThemeData> {
        if let Some(theme) = builtin::find(id) {
            let parsed = parse_theme(theme.source, theme.id)?;
            return Ok(data(id.into(), parsed, true));
        }
        let slug = id
            .strip_prefix(USER_PREFIX)
            .filter(|slug| is_valid_slug(slug))
            .ok_or_else(|| AppError::NotFound(format!("theme \"{id}\"")))?;
        let path = self.dir.join(format!("{slug}.json"));
        if !path.is_file() {
            return Err(AppError::NotFound(format!("theme \"{id}\"")));
        }
        Ok(data(id.into(), read_theme(&path, slug)?, false))
    }

    /// Validates `contents` and saves it. Re-importing a theme with the same name replaces it.
    pub fn import(&self, file_name: &str, contents: &str) -> AppResult<ThemeSummary> {
        if contents.len() > MAX_IMPORT_BYTES {
            return Err(AppError::InvalidTheme("file is larger than 4 MB".into()));
        }
        let parsed = parse_theme(contents, file_stem(file_name))?;
        let slug = slugify(&parsed.name);
        let mut bytes = serde_json::to_vec_pretty(&parsed.document)?;
        bytes.push(b'\n');
        write_atomic(&self.dir.join(format!("{slug}.json")), &bytes)?;
        tracing::info!("imported theme \"{}\" as {USER_PREFIX}{slug}", parsed.name);
        Ok(summary(format!("{USER_PREFIX}{slug}"), &parsed, false))
    }

    fn user_files(&self) -> Vec<(String, PathBuf)> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| {
                let slug = path.file_stem()?.to_str()?.to_owned();
                is_valid_slug(&slug).then_some((slug, path))
            })
            .collect()
    }
}

fn read_theme(path: &Path, fallback_name: &str) -> AppResult<ParsedTheme> {
    parse_theme(&fs::read_to_string(path)?, fallback_name)
}

fn summary(id: String, theme: &ParsedTheme, builtin: bool) -> ThemeSummary {
    ThemeSummary {
        id,
        name: theme.name.clone(),
        kind: theme.kind,
        builtin,
    }
}

fn data(id: String, theme: ParsedTheme, builtin: bool) -> ThemeData {
    ThemeData {
        summary: summary(id, &theme, builtin),
        colors: theme.colors,
    }
}

fn file_stem(file_name: &str) -> &str {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    base.strip_suffix(".json").unwrap_or(base)
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug
        .trim_end_matches('-')
        .chars()
        .take(MAX_SLUG_CHARS)
        .collect();
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        // Names without ASCII letters (e.g. CJK) still get a stable, distinct slug.
        format!("theme-{:08x}", fnv1a(name))
    } else {
        slug.into()
    }
}

fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= MAX_SLUG_CHARS
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::ThemeKind;
    use crate::util::test_dir::TestDir;

    const MINI_THEME: &str = r##"{
        // exported from VS Code
        "name": "Mini Light",
        "type": "light",
        "colors": { "editor.background": "#ffffff", "editor.foreground": "#222222" },
        "tokenColors": [],
    }"##;

    #[test]
    fn lists_builtins_first() {
        let dir = TestDir::new("themes-list");
        let store = ThemeStore::new(dir.path().join("themes"));
        let ids: Vec<_> = store.list().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, ["agentflow-dark", "agentflow-light", "retro-crt"]);
    }

    #[test]
    fn import_saves_lists_and_loads() {
        let dir = TestDir::new("themes-import");
        let store = ThemeStore::new(dir.path().join("themes"));

        let imported = store
            .import("C:\\themes\\mini.json", MINI_THEME)
            .expect("import");
        assert_eq!(imported.id, "user:mini-light");
        assert_eq!(imported.kind, ThemeKind::Light);
        assert!(!imported.builtin);

        let listed = store.list();
        assert_eq!(listed.len(), BUILTIN_THEMES.len() + 1);
        assert_eq!(listed.last(), Some(&imported));

        let loaded = store.get("user:mini-light").expect("get");
        assert_eq!(loaded.colors["editor.foreground"], "#222222");
    }

    #[test]
    fn rejects_bad_imports_and_unknown_ids() {
        let dir = TestDir::new("themes-bad");
        let store = ThemeStore::new(dir.path().join("themes"));
        assert!(matches!(
            store.import("x.json", "{}"),
            Err(AppError::InvalidTheme(_))
        ));
        for id in ["nope", "user:../secret", "user:", "user:UPPER"] {
            assert!(matches!(store.get(id), Err(AppError::NotFound(_))), "{id}");
        }
    }

    #[test]
    fn skips_broken_user_files() {
        let dir = TestDir::new("themes-broken");
        let themes_dir = dir.path().join("themes");
        fs::create_dir_all(&themes_dir).expect("mkdir");
        fs::write(themes_dir.join("broken.json"), "{").expect("write");
        let store = ThemeStore::new(themes_dir);
        assert_eq!(store.list().len(), BUILTIN_THEMES.len());
    }

    #[test]
    fn slugs_are_safe() {
        assert_eq!(slugify("One Dark Pro!"), "one-dark-pro");
        assert!(slugify("  ---  ").starts_with("theme-"));
        assert_ne!(slugify("主题"), slugify("夜间"));
        assert_eq!(slugify("Ünïcødé 主题"), "n-c-d");
        assert!(slugify(&"a".repeat(100)).len() <= MAX_SLUG_CHARS);
        assert_eq!(file_stem("/home/me/Night Owl.json"), "Night Owl");
    }
}
