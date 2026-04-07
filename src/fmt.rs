use regex::Regex;
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[^m]*m|\x1b\]8;;[^\x07]*\x07").unwrap());

pub fn visible_len(s: &str) -> usize {
    ANSI_RE.replace_all(s, "").chars().count()
}

#[cfg(test)]
pub fn dot_bar(pct: u32, width: usize) -> (String, String) {
    use crate::theme::{ansi, ThemePalette, RESET};
    let p = ThemePalette::default();
    let pct = pct.min(100);
    let filled = ((pct as usize * width + 50) / 100).min(width);
    let empty = width - filled;
    let col = if pct >= 80 {
        ansi(p.danger)
    } else if pct >= 50 {
        ansi(p.warning)
    } else {
        ansi(p.success)
    };
    let muted = ansi(p.muted);
    let bar = format!(
        "{col}{filled}{muted}{empty}{reset}",
        filled = "●".repeat(filled),
        empty = "○".repeat(empty),
        reset = RESET,
    );
    (bar, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ansi, ThemePalette};

    #[test]
    fn dot_bar_zero() {
        let p = ThemePalette::default();
        let (bar, col) = dot_bar(0, 10);
        assert_eq!(col, ansi(p.success));
        assert!(bar.contains(&"○".repeat(10)));
        assert!(!bar.contains('●'));
    }

    #[test]
    fn dot_bar_50() {
        let p = ThemePalette::default();
        let (bar, col) = dot_bar(50, 10);
        assert_eq!(col, ansi(p.warning));
        assert!(bar.contains(&"●".repeat(5)));
        assert!(bar.contains(&"○".repeat(5)));
    }

    #[test]
    fn dot_bar_80() {
        let p = ThemePalette::default();
        let (bar, col) = dot_bar(80, 10);
        assert_eq!(col, ansi(p.danger));
        assert!(bar.contains(&"●".repeat(8)));
        assert!(bar.contains(&"○".repeat(2)));
    }

    #[test]
    fn dot_bar_100() {
        let p = ThemePalette::default();
        let (bar, col) = dot_bar(100, 10);
        assert_eq!(col, ansi(p.danger));
        assert!(bar.contains(&"●".repeat(10)));
        assert!(!bar.contains('○'));
    }

    #[test]
    fn test_visible_len_plain() {
        assert_eq!(visible_len("hello"), 5);
    }

    #[test]
    fn test_visible_len_ansi() {
        use crate::theme::{ansi, ThemePalette, RESET};
        let p = ThemePalette::default();
        let s = format!("{}hello{}", ansi(p.success), RESET);
        assert_eq!(visible_len(&s), 5);
    }

    #[test]
    fn test_visible_len_osc8() {
        let s = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07";
        assert_eq!(visible_len(s), 5);
    }
}
