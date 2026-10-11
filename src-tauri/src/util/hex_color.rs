//! Validation for the hex color formats VS Code themes use.

/// Accepts `#rgb`, `#rgba`, `#rrggbb` and `#rrggbbaa`.
pub fn is_hex_color(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    matches!(digits.len(), 3 | 4 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::is_hex_color;

    #[test]
    fn accepts_vscode_formats() {
        for ok in ["#abc", "#abcd", "#A1B2C3", "#a1b2c3d4"] {
            assert!(is_hex_color(ok), "{ok} should be valid");
        }
    }

    #[test]
    fn rejects_other_values() {
        for bad in [
            "",
            "#",
            "abc",
            "#ab",
            "#abcde",
            "#ggg",
            "red",
            "rgb(0,0,0)",
            "#1234567",
        ] {
            assert!(!is_hex_color(bad), "{bad} should be invalid");
        }
    }
}
