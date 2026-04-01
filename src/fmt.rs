use crate::theme::{BarStyle, IconsConfig, Theme};
use regex::Regex;
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[^m]*m|\x1b\]8;;[^\x07]*\x07").unwrap());

pub fn visible_len(s: &str) -> usize {
    ANSI_RE.replace_all(s, "").chars().count()
}

#[cfg(test)]
pub fn dot_bar(pct: u32, width: usize) -> (String, String) {
    let theme = Theme::default();
    let pct = pct.min(100);
    let filled = ((pct as usize * width + 50) / 100).min(width);
    let empty = width - filled;
    let col = if pct >= 80 {
        &theme.red
    } else if pct >= 50 {
        &theme.orange
    } else {
        &theme.green
    };
    let bar = format!(
        "{col}{filled}{dim}{empty}{reset}",
        filled = "●".repeat(filled),
        dim = theme.dim,
        empty = "○".repeat(empty),
        reset = theme.reset,
    );
    (bar, col.to_string())
}

pub fn fmt_tokens(n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    if n >= 1_000_000 {
        return format!("{:.1}m", n as f64 / 1_000_000.0);
    }
    if n >= 1_000 {
        return format!("{}k", n / 1000);
    }
    n.to_string()
}

pub fn fmt_cost(usd: f64) -> String {
    if usd <= 0.0 {
        return "$0".into();
    }
    if usd >= 0.01 {
        format!("${:.2}", usd)
    } else {
        format!("${:.4}", usd)
    }
}

pub fn fmt_duration(ms: u64) -> String {
    let s = ms / 1000;
    if s == 0 {
        return "0s".into();
    }
    if s < 60 {
        return format!("{}s", s);
    }
    if s < 3600 {
        return format!("{}m{:02}s", s / 60, s % 60);
    }
    format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
}

pub fn superscript(s: &str) -> String {
    const FROM: &str = "0123456789.abcdefghijklmnoprstuvwxyz";
    const TO: &[char] = &[
        '⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '·', 'ᵃ', 'ᵇ', 'ᶜ', 'ᵈ', 'ᵉ', 'ᶠ', 'ᵍ',
        'ʰ', 'ⁱ', 'ʲ', 'ᵏ', 'ˡ', 'ᵐ', 'ⁿ', 'ᵒ', 'ᵖ', 'ʳ', 'ˢ', 'ᵗ', 'ᵘ', 'ᵛ', 'ʷ', 'ˣ', 'ʸ', 'ᶻ',
    ];
    s.chars()
        .map(|c| FROM.find(c).and_then(|i| TO.get(i).copied()).unwrap_or(c))
        .collect()
}

pub fn subscript(s: &str) -> String {
    const FROM: &str = "0123456789.aehijklmnoprstuvx";
    const TO: &[char] = &[
        '₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉', '.', 'ₐ', 'ₑ', 'ₕ', 'ᵢ', 'ⱼ', 'ₖ', 'ₗ',
        'ₘ', 'ₙ', 'ₒ', 'ₚ', 'ᵣ', 'ₛ', 'ₜ', 'ᵤ', 'ᵥ', 'ₓ',
    ];
    s.chars()
        .map(|c| FROM.find(c).and_then(|i| TO.get(i).copied()).unwrap_or(c))
        .collect()
}

const SEG_DIGITS: &[char] = &['🯰', '🯱', '🯲', '🯳', '🯴', '🯵', '🯶', '🯷', '🯸', '🯹'];

pub fn seg_pct(n: u32, col: &str, theme: &Theme) -> String {
    let v = n.min(999);
    let digits: String = v
        .to_string()
        .chars()
        .map(|c| SEG_DIGITS[c.to_digit(10).unwrap() as usize])
        .collect();
    format!("{col}{digits}٪{}", theme.reset)
}

/// Gradient context bar. Returns (bar_string, label_color).
pub fn context_bar(
    pct: u32,
    width: usize,
    theme: &Theme,
    icons: &IconsConfig,
    bar_style: &BarStyle,
) -> (String, String) {
    let pct = pct.min(100);
    let (default_fill, default_empty, default_half) = match bar_style {
        BarStyle::Block => ('■', '□', '◧'),
        BarStyle::Dot => ('●', '○', '◐'),
        BarStyle::Ascii => ('#', '-', '~'),
    };
    let fill_ch = icons.bar_fill.unwrap_or(default_fill);
    let empty_ch = icons.bar_empty.unwrap_or(default_empty);
    let half_ch = icons.bar_half.unwrap_or(default_half);
    // Scale default thresholds (4 and 9 out of 12) proportionally.
    let threshold0 = (4 * width + 6) / 12;
    let threshold1 = (9 * width + 6) / 12;

    let fill_f = pct as f64 / 100.0 * width as f64;
    let fill_int = fill_f.floor() as usize;
    let frac = fill_f - fill_int as f64;

    let mut bar = String::with_capacity(width * 16);

    for pos in 0..width {
        let (bright, dim) = if pos < threshold0 {
            (&theme.green, &theme.dim_green)
        } else if pos < threshold1 {
            (&theme.orange, &theme.dim_orange)
        } else {
            (&theme.red, &theme.dim_red)
        };
        let half = if pos < threshold0 {
            threshold0 / 2
        } else if pos < threshold1 {
            threshold0 + (threshold1 - threshold0) / 2
        } else {
            threshold1 + (width - threshold1) / 2
        };
        let col = if pos >= half { bright } else { dim };

        if pos < fill_int {
            bar.push_str(col);
            bar.push(fill_ch);
        } else if pos == fill_int && frac > 0.0 {
            let ch = if frac < 0.5 { half_ch } else { fill_ch };
            bar.push_str(col);
            bar.push(ch);
        } else {
            bar.push_str(&theme.dim);
            bar.push(empty_ch);
        }
    }
    bar.push_str(&theme.reset);

    let label_col = if fill_int >= threshold1 {
        theme.red.clone()
    } else if fill_int >= threshold0 {
        theme.orange.clone()
    } else {
        theme.green.clone()
    };

    (bar, label_col)
}

/// Quota usage bar with optional pace marker. Returns (bar_string, label_color).
pub fn usage_bar(
    pct: u32,
    width: usize,
    col: &str,
    pace_pct: Option<f64>,
    theme: &Theme,
    bar_style: &BarStyle,
    icons: &IconsConfig,
) -> (String, String) {
    let pct = pct.min(100);
    let fill_f = pct as f64 / 100.0 * width as f64;
    let fill_int = fill_f.floor() as usize;
    let frac = fill_f - fill_int as f64;

    let pace_seg = pace_pct.map(|p| (p / 100.0 * width as f64).floor() as usize);

    // Pace marker color
    let pace_col: &str = if let Some(p) = pace_pct {
        let ratio = if p > 0.0 {
            pct as f64 / p
        } else {
            f64::INFINITY
        };
        if ratio < 0.8 {
            &theme.red
        } else if ratio < 1.0 {
            &theme.orange
        } else {
            &theme.dim
        }
    } else {
        &theme.dim
    };

    let empty_ch = match bar_style {
        BarStyle::Ascii => icons.quota_empty.unwrap_or('-'),
        _ => icons.quota_empty.unwrap_or('○'),
    };
    let pace_ch = icons.quota_pace.unwrap_or('◌');

    let mut bar = String::with_capacity(width * 16);

    for pos in 0..width {
        let is_pre_pace = pace_seg
            .map(|ps| pos < ps && fill_int > ps)
            .unwrap_or(false);
        let effective_col = if is_pre_pace { &theme.dim } else { col };

        if pos < fill_int {
            let ch = quota_fill_char(bar_style, icons, pos as f64 / fill_f);
            bar.push_str(effective_col);
            bar.push(ch);
        } else if pos == fill_int && frac > 0.0 {
            let ch = quota_fill_char(bar_style, icons, pos as f64 / fill_f.max(1.0));
            bar.push_str(effective_col);
            bar.push(ch);
        } else {
            let is_pace_pos = pace_seg.map(|ps| pos == ps).unwrap_or(false);
            if is_pace_pos {
                bar.push_str(pace_col);
                bar.push(pace_ch);
            } else {
                bar.push_str(&theme.dim);
                bar.push(empty_ch);
            }
        }
    }
    bar.push_str(&theme.reset);

    let label_col = if pct >= 80 {
        theme.red.clone()
    } else if pct >= 50 {
        theme.orange.clone()
    } else {
        theme.cyan.clone()
    };

    (bar, label_col)
}

fn quota_fill_char(bar_style: &BarStyle, icons: &IconsConfig, zone_f: f64) -> char {
    if let Some(ch) = icons.quota_fill {
        return ch;
    }
    match bar_style {
        BarStyle::Block => {
            if zone_f < 0.33 {
                '◎'
            } else if zone_f < 0.66 {
                '◉'
            } else {
                '●'
            }
        }
        BarStyle::Dot => '●',
        BarStyle::Ascii => '#',
    }
}

fn _fmt_duration(secs: f64, show_seconds: bool) -> String {
    let secs = secs as u64;
    if show_seconds && secs < 60 {
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;
    if days > 0 {
        format!("{}d {}h", days, hours % 24)
    } else if hours > 0 {
        format!("{}h {:02}m", hours, mins % 60)
    } else {
        format!("{}m", mins)
    }
}

/// Format a reset countdown from a JSON value (epoch seconds or ISO 8601 string).
/// Returns (formatted_string, remaining_secs).
pub fn fmt_reset(resets_at: &serde_json::Value) -> (String, f64) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let target = match resets_at {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => {
            // Parse ISO 8601 via chrono-style manual parse or simple epoch extraction.
            // We do a minimal parse: try to parse as f64 first, then as RFC3339.
            if let Ok(v) = s.parse::<f64>() {
                v
            } else {
                // Minimal ISO 8601 parser: YYYY-MM-DDTHH:MM:SSZ
                parse_iso8601(s).unwrap_or(0.0)
            }
        }
        _ => 0.0,
    };

    let remaining = (target - now).max(0.0);
    let formatted = _fmt_duration(remaining, false);
    (formatted, remaining)
}

fn parse_iso8601(s: &str) -> Option<f64> {
    // Minimal parser for YYYY-MM-DDTHH:MM:SS[.fff][Z|+HH:MM]
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: i64 = s[5..7].parse().ok()?;
    let day: i64 = s[8..10].parse().ok()?;
    let hour: i64 = s[11..13].parse().ok()?;
    let min: i64 = s[14..16].parse().ok()?;
    let sec: i64 = s[17..19].parse().ok()?;

    // Days since epoch via rough formula (good enough for near-future timestamps)
    let days = days_since_epoch(year, month, day)?;
    let epoch_secs = days * 86400 + hour * 3600 + min * 60 + sec;

    // Timezone offset
    let offset_secs = if s.len() > 19 {
        let tz = &s[19..];
        if tz.starts_with('Z') || tz.starts_with('z') {
            0
        } else if tz.starts_with('+') || tz.starts_with('-') {
            let sign: i64 = if tz.starts_with('-') { -1 } else { 1 };
            let tz = &tz[1..];
            let parts: Vec<&str> = tz.splitn(2, ':').collect();
            let h: i64 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
            let m: i64 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            sign * (h * 3600 + m * 60)
        } else {
            0
        }
    } else {
        0
    };

    Some((epoch_secs - offset_secs) as f64)
}

fn days_since_epoch(year: i64, month: i64, day: i64) -> Option<i64> {
    // Rata Die algorithm
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468; // offset to Unix epoch
    Some(days)
}

/// Compute pace balance in seconds.
/// Returns None if elapsed time is less than 60 seconds.
pub fn pace_balance_secs(used: f64, remaining_secs: f64, window_secs: f64) -> Option<i64> {
    let elapsed = window_secs - remaining_secs;
    if elapsed < 60.0 {
        return None;
    }
    let balance_pct = (100.0 - used) - (remaining_secs / window_secs * 100.0);
    Some((balance_pct * window_secs / 100.0).round() as i64)
}

/// Format pace as italic colored segmented hours.
pub fn fmt_pace(secs: i64, window_secs: u64, theme: &Theme) -> String {
    let col: &str = if secs >= 0 {
        &theme.dim_cyan
    } else {
        let deficit_pct = secs.unsigned_abs() as f64 / window_secs as f64 * 100.0;
        if deficit_pct >= 15.0 {
            &theme.dim_red
        } else if deficit_pct >= 8.0 {
            &theme.dim_orange
        } else {
            &theme.dim_yellow
        }
    };
    let sign = if secs >= 0 { "+" } else { "-" };
    let hours = secs.unsigned_abs() / 3600;
    let seg_hours = hours
        .to_string()
        .chars()
        .map(|c| SEG_DIGITS[c.to_digit(10).unwrap() as usize])
        .collect::<String>();
    format!("{}{col}{sign}{seg_hours}h{}", theme.italic, theme.reset)
}

/// Determine quota color based on utilization and remaining time.
pub fn quota_color(
    utilization: f64,
    remaining_secs: f64,
    window_secs: f64,
    theme: &Theme,
) -> String {
    if remaining_secs <= 0.0 || window_secs <= 0.0 {
        // No time context -- simple threshold
        if utilization >= 80.0 {
            theme.red.clone()
        } else if utilization >= 50.0 {
            theme.orange.clone()
        } else {
            theme.cyan.clone()
        }
    } else {
        let elapsed = window_secs - remaining_secs;
        if elapsed <= 0.0 {
            return theme.cyan.clone();
        }
        let even_pace_used = elapsed / window_secs * 100.0;
        let per_unit_remaining = if even_pace_used > 0.0 {
            (100.0 - utilization) / (100.0 - even_pace_used)
        } else {
            1.0
        };
        if per_unit_remaining >= 0.70 {
            theme.cyan.clone()
        } else if per_unit_remaining >= 0.35 {
            theme.orange.clone()
        } else {
            theme.red.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ported from jarvis fmt.rs ----

    #[test]
    fn dot_bar_zero() {
        let t = Theme::default();
        let (bar, col) = dot_bar(0, 10);
        assert_eq!(col, t.green);
        assert!(bar.contains(&"○".repeat(10)));
        assert!(!bar.contains('●'));
    }

    #[test]
    fn dot_bar_50() {
        let t = Theme::default();
        let (bar, col) = dot_bar(50, 10);
        assert_eq!(col, t.orange);
        assert!(bar.contains(&"●".repeat(5)));
        assert!(bar.contains(&"○".repeat(5)));
    }

    #[test]
    fn dot_bar_80() {
        let t = Theme::default();
        let (bar, col) = dot_bar(80, 10);
        assert_eq!(col, t.red);
        assert!(bar.contains(&"●".repeat(8)));
        assert!(bar.contains(&"○".repeat(2)));
    }

    #[test]
    fn dot_bar_100() {
        let t = Theme::default();
        let (bar, col) = dot_bar(100, 10);
        assert_eq!(col, t.red);
        assert!(bar.contains(&"●".repeat(10)));
        assert!(!bar.contains('○'));
    }

    #[test]
    fn test_fmt_tokens() {
        assert_eq!(fmt_tokens(0), "0");
        assert_eq!(fmt_tokens(999), "999");
        assert_eq!(fmt_tokens(1000), "1k");
        assert_eq!(fmt_tokens(1_500_000), "1.5m");
    }

    #[test]
    fn test_fmt_cost() {
        assert_eq!(fmt_cost(0.0), "$0");
        assert_eq!(fmt_cost(1.234), "$1.23");
        assert_eq!(fmt_cost(0.001), "$0.0010");
    }

    #[test]
    fn test_fmt_duration() {
        assert_eq!(fmt_duration(0), "0s");
        assert_eq!(fmt_duration(45_000), "45s");
        assert_eq!(fmt_duration(754_000), "12m34s");
        assert_eq!(fmt_duration(7_500_000), "2h05m");
    }

    #[test]
    fn test_superscript() {
        let result = superscript("2.1.34");
        assert_eq!(result, "²·¹·³⁴");
    }

    #[test]
    fn test_subscript() {
        let result = subscript("claude opus");
        assert_eq!(result, "cₗₐᵤdₑ ₒₚᵤₛ");
    }

    // ---- new functions ----

    #[test]
    fn test_seg_pct_zero() {
        let t = Theme::default();
        let s = seg_pct(0, &t.green, &t);
        assert!(s.contains('🯰'));
        assert!(s.contains('٪'));
        assert!(s.contains(&t.green));
        assert!(s.contains(&t.reset));
    }

    #[test]
    fn test_seg_pct_clamp() {
        let t = Theme::default();
        let s = seg_pct(1000, &t.red, &t);
        // clamped to 999
        assert!(s.contains('🯹'));
    }

    #[test]
    fn test_visible_len_plain() {
        assert_eq!(visible_len("hello"), 5);
    }

    #[test]
    fn test_visible_len_ansi() {
        let t = Theme::default();
        let s = format!("{}hello{}", t.green, t.reset);
        assert_eq!(visible_len(&s), 5);
    }

    #[test]
    fn test_visible_len_osc8() {
        // OSC-8 hyperlink: ESC]8;;url ST text ESC]8;; ST
        let s = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07";
        assert_eq!(visible_len(s), 5);
    }

    #[test]
    fn context_bar_zero() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, col) = context_bar(0, 12, &t, &i, &BarStyle::default());
        assert_eq!(col, t.green);
        assert!(bar.contains('□'));
        assert!(!bar.contains('■'));
    }

    #[test]
    fn context_bar_full() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, col) = context_bar(100, 12, &t, &i, &BarStyle::default());
        assert_eq!(col, t.red);
        assert!(bar.contains('■'));
        assert!(!bar.contains('□'));
    }

    #[test]
    fn context_bar_small_width_label() {
        let t = Theme::default();
        let i = IconsConfig::default();
        // width=4, pct=80 -> fill_int=3, threshold1=(9*4+6)/12=3 -> RED
        let (_bar, col) = context_bar(80, 4, &t, &i, &BarStyle::default());
        assert_eq!(col, t.red);
    }

    #[test]
    fn context_bar_mid_width_label() {
        let t = Theme::default();
        let i = IconsConfig::default();
        // width=8, pct=50 -> fill_int=4, threshold0=(4*8+6)/12=3 -> ORANGE
        let (_bar, col) = context_bar(50, 8, &t, &i, &BarStyle::default());
        assert_eq!(col, t.orange);
    }

    #[test]
    fn context_bar_partial() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = context_bar(50, 12, &t, &i, &BarStyle::default());
        assert!(bar.contains('■') || bar.contains('◧'));
        assert!(bar.contains('□'));
    }

    #[test]
    fn context_bar_custom_fill_chars() {
        let t = Theme::default();
        let icons = IconsConfig {
            bar_fill: Some('█'),
            bar_empty: Some('░'),
            bar_half: Some('▒'),
            ..IconsConfig::default()
        };
        let (bar, _col) = context_bar(50, 12, &t, &icons, &BarStyle::default());
        assert!(bar.contains('█') || bar.contains('▒'));
        assert!(bar.contains('░'));
        assert!(!bar.contains('■'));
        assert!(!bar.contains('□'));
    }

    #[test]
    fn context_bar_dot_style() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = context_bar(50, 12, &t, &i, &BarStyle::Dot);
        assert!(bar.contains('●') || bar.contains('◐'));
        assert!(bar.contains('○'));
        assert!(!bar.contains('■'));
        assert!(!bar.contains('□'));
    }

    #[test]
    fn context_bar_ascii_style() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = context_bar(50, 12, &t, &i, &BarStyle::Ascii);
        assert!(bar.contains('#') || bar.contains('~'));
        assert!(bar.contains('-'));
        assert!(!bar.contains('■'));
        assert!(!bar.contains('□'));
    }

    #[test]
    fn usage_bar_no_pace() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = usage_bar(50, 10, &t.cyan, None, &t, &BarStyle::default(), &i);
        assert!(!bar.contains('◌'));
    }

    #[test]
    fn usage_bar_with_pace() {
        let t = Theme::default();
        let i = IconsConfig::default();
        // fill at 80%, pace at 50% -- ahead of pace, should show no pace marker in filled zone
        let (bar, _col) = usage_bar(80, 10, &t.cyan, Some(50.0), &t, &BarStyle::default(), &i);
        // pace marker should not appear (it's in filled zone)
        assert!(bar.contains('●') || bar.contains('◉') || bar.contains('◎'));
    }

    #[test]
    fn usage_bar_pace_marker_visible() {
        let t = Theme::default();
        let i = IconsConfig::default();
        // fill at 20%, pace at 60% -- behind pace, marker should appear
        let (bar, _col) = usage_bar(20, 10, &t.cyan, Some(60.0), &t, &BarStyle::default(), &i);
        assert!(bar.contains('◌'));
    }

    #[test]
    fn usage_bar_ascii_style() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = usage_bar(50, 10, &t.cyan, None, &t, &BarStyle::Ascii, &i);
        assert!(bar.contains('#'));
        assert!(bar.contains('-'));
        assert!(!bar.contains('●'));
        assert!(!bar.contains('○'));
    }

    #[test]
    fn usage_bar_dot_style() {
        let t = Theme::default();
        let i = IconsConfig::default();
        let (bar, _col) = usage_bar(50, 10, &t.cyan, None, &t, &BarStyle::Dot, &i);
        assert!(bar.contains('●'));
        assert!(bar.contains('○'));
        // Dot style should NOT have density progression chars
        assert!(!bar.contains('◎'));
        assert!(!bar.contains('◉'));
    }

    #[test]
    fn usage_bar_custom_quota_fill() {
        let t = Theme::default();
        let icons = IconsConfig {
            quota_fill: Some('X'),
            quota_empty: Some('.'),
            ..IconsConfig::default()
        };
        let (bar, _col) = usage_bar(50, 10, &t.cyan, None, &t, &BarStyle::Block, &icons);
        assert!(bar.contains('X'));
        assert!(bar.contains('.'));
        assert!(!bar.contains('●'));
        assert!(!bar.contains('○'));
    }

    #[test]
    fn test_pace_balance_too_early() {
        assert!(pace_balance_secs(50.0, 3599.0, 3600.0).is_none());
    }

    #[test]
    fn test_pace_balance_at_pace() {
        // 50% used, 50% remaining, 1h window -> at pace -> balance ~= 0
        let b = pace_balance_secs(50.0, 1800.0, 3600.0).unwrap();
        assert_eq!(b, 0);
    }

    #[test]
    fn test_quota_color_no_time() {
        let t = Theme::default();
        assert_eq!(quota_color(85.0, 0.0, 0.0, &t), t.red);
        assert_eq!(quota_color(60.0, 0.0, 0.0, &t), t.orange);
        assert_eq!(quota_color(30.0, 0.0, 0.0, &t), t.cyan);
    }
}
