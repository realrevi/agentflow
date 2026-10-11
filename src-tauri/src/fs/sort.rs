//! Natural, case-insensitive ordering: `file2` before `file10`, folders before files.

use std::cmp::Ordering;
use std::iter::Peekable;
use std::str::Chars;

use super::model::FsEntry;

pub fn sort_entries(entries: &mut [FsEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| natural_cmp(&a.name, &b.name))
    });
}

/// Compares runs of digits by value and everything else case-insensitively,
/// falling back to a plain comparison so the order is total and stable.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut left, mut right) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let order = compare_numbers(&digits(&mut left), &digits(&mut right));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                left.next();
                right.next();
            }
        }
    }
}

fn digits(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(c) = chars.next_if(char::is_ascii_digit) {
        run.push(c);
    }
    run
}

/// Numeric comparison of digit strings of any length (no overflow).
fn compare_numbers(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> FsEntry {
        FsEntry {
            name: name.into(),
            path: name.into(),
            is_dir,
            is_symlink: false,
        }
    }

    #[test]
    fn numbers_compare_by_value() {
        let mut names = vec!["file10.txt", "file2.txt", "File1.txt", "file02.txt", "a"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            ["a", "File1.txt", "file02.txt", "file2.txt", "file10.txt"]
        );
        assert_eq!(
            natural_cmp("v99999999999999999999", "v100000000000000000000"),
            Ordering::Less
        );
    }

    #[test]
    fn folders_come_first() {
        let mut entries = vec![
            entry("b.rs", false),
            entry("src", true),
            entry("A.md", false),
            entry(".github", true),
        ];
        sort_entries(&mut entries);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, [".github", "src", "A.md", "b.rs"]);
    }
}
