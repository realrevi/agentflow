//! Fuzzy matching for quick open. Query characters must appear in order in the path;
//! the match is anchored as far right as possible (towards the file name) and scored
//! with bonuses for word starts, consecutive runs and hits in the file name.

const MATCH: i64 = 16;
const BOUNDARY: i64 = 24;
const CAMEL: i64 = 12;
const CONSECUTIVE: i64 = 20;
const IN_NAME: i64 = 10;
const GAP: i64 = 2;
const MAX_GAP_PENALTY: i64 = 24;
const NAME_EXACT: i64 = 100;
const NAME_PREFIX: i64 = 50;

pub struct Query {
    chars: Vec<char>,
}

impl Query {
    /// Case-insensitive; whitespace is ignored and `\` matches `/`.
    pub fn new(text: &str) -> Self {
        let chars = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| if c == '\\' { '/' } else { fold(c) })
            .collect();
        Self { chars }
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Cheap pre-check without allocating: is the query a subsequence of `path`?
    pub fn matches(&self, path: &str) -> bool {
        let mut wanted = self.chars.iter().peekable();
        for c in path.chars() {
            if wanted.peek().is_some_and(|&&w| w == fold(c)) {
                wanted.next();
            }
        }
        wanted.peek().is_none()
    }

    /// The match score and the matched char positions, or `None` if there is no match.
    pub fn score(&self, path: &str) -> Option<(i64, Vec<u32>)> {
        if self.is_empty() || !self.matches(path) {
            return None;
        }
        let chars: Vec<char> = path.chars().collect();
        let lower: Vec<char> = chars.iter().map(|&c| fold(c)).collect();
        let positions = self.positions(&lower)?;
        let name_start = chars.iter().rposition(|&c| c == '/').map_or(0, |i| i + 1);

        let mut score = 0;
        let mut previous: Option<usize> = None;
        for &position in &positions {
            let p = position as usize;
            score += MATCH;
            match p.checked_sub(1).map(|i| chars[i]) {
                None => score += BOUNDARY,
                Some(before) if is_word_separator(before) => score += BOUNDARY,
                Some(before) if before.is_lowercase() && chars[p].is_uppercase() => score += CAMEL,
                Some(_) => {}
            }
            if let Some(q) = previous {
                if p == q + 1 {
                    score += CONSECUTIVE;
                } else {
                    let gap = i64::try_from(p - q - 1).unwrap_or(i64::MAX);
                    score -= gap.saturating_mul(GAP).min(MAX_GAP_PENALTY);
                }
            }
            if p >= name_start {
                score += IN_NAME;
            }
            previous = Some(p);
        }
        let name = &lower[name_start..];
        if name == self.chars.as_slice() {
            score += NAME_EXACT;
        } else if name.starts_with(&self.chars) {
            score += NAME_PREFIX;
        }
        Some((score, positions))
    }

    /// Rightmost possible start (found scanning backwards), then a greedy forward match.
    fn positions(&self, lower: &[char]) -> Option<Vec<u32>> {
        let mut remaining = self.chars.len();
        let mut start = None;
        for (i, &c) in lower.iter().enumerate().rev() {
            if remaining > 0 && c == self.chars[remaining - 1] {
                remaining -= 1;
                if remaining == 0 {
                    start = Some(i);
                    break;
                }
            }
        }
        let mut positions = Vec::with_capacity(self.chars.len());
        let mut wanted = self.chars.iter().peekable();
        for (i, &c) in lower.iter().enumerate().skip(start?) {
            if wanted.peek().is_some_and(|&&w| w == c) {
                wanted.next();
                positions.push(u32::try_from(i).ok()?);
            }
        }
        Some(positions)
    }
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn is_word_separator(c: char) -> bool {
    matches!(c, '/' | '\\' | '_' | '-' | '.' | ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best<'a>(query: &str, paths: &[&'a str]) -> Vec<&'a str> {
        let query = Query::new(query);
        let mut scored: Vec<_> = paths
            .iter()
            .filter_map(|path| query.score(path).map(|(score, _)| (score, *path)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.len().cmp(&b.1.len())));
        scored.into_iter().map(|(_, path)| path).collect()
    }

    #[test]
    fn requires_an_ordered_subsequence() {
        let query = Query::new("mrs");
        assert!(query.matches("src/main.rs"));
        assert!(!Query::new("srm").matches("main.rs"));
        assert_eq!(Query::new("xyz").score("src/main.rs"), None);
    }

    #[test]
    fn prefers_file_names_and_word_starts() {
        let paths = [
            "src/features/layout/layoutStore.ts",
            "src/layouts/main/store.rs",
            "docs/LICENSE",
            "src/lib.rs",
        ];
        assert_eq!(best("lib", &paths)[0], "src/lib.rs");
        assert_eq!(
            best("layoutstore", &paths)[0],
            "src/features/layout/layoutStore.ts"
        );
        assert_eq!(best("LS", &paths)[0], "src/features/layout/layoutStore.ts");
    }

    #[test]
    fn reports_char_positions_for_highlighting() {
        let (_, positions) = Query::new("main").score("src/main.rs").expect("match");
        assert_eq!(positions, [4, 5, 6, 7]);
        let (_, positions) = Query::new("é").score("café.md").expect("match");
        assert_eq!(positions, [3], "positions count chars, not bytes");
    }
}
