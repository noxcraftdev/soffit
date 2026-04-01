use std::fmt;
use std::str::FromStr;

/// Runtime theme: fully resolved ANSI escape strings.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub green: String,
    pub orange: String,
    pub red: String,
    pub dim: String,
    pub lgray: String,
    pub cyan: String,
    pub purple: String,
    pub yellow: String,
    pub reset: String,
    pub dim_green: String,
    pub dim_yellow: String,
    pub dim_orange: String,
    pub dim_red: String,
    pub dim_cyan: String,
    pub dim_pink: String,
    pub italic: String,
    pub no_italic: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            green: "\x1b[38;5;114m".into(),
            orange: "\x1b[38;5;215m".into(),
            red: "\x1b[38;5;203m".into(),
            dim: "\x1b[38;5;242m".into(),
            lgray: "\x1b[38;5;250m".into(),
            cyan: "\x1b[38;5;111m".into(),
            purple: "\x1b[38;5;183m".into(),
            yellow: "\x1b[38;5;228m".into(),
            reset: "\x1b[0m".into(),
            dim_green: "\x1b[38;5;65m".into(),
            dim_yellow: "\x1b[38;5;136m".into(),
            dim_orange: "\x1b[38;5;130m".into(),
            dim_red: "\x1b[38;5;131m".into(),
            dim_cyan: "\x1b[38;5;67m".into(),
            dim_pink: "\x1b[38;5;175m".into(),
            italic: "\x1b[3m".into(),
            no_italic: "\x1b[23m".into(),
        }
    }
}

/// User-facing config: 256-color indices. `None` means "use default".
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    pub green: Option<u8>,
    pub orange: Option<u8>,
    pub red: Option<u8>,
    pub dim: Option<u8>,
    pub lgray: Option<u8>,
    pub cyan: Option<u8>,
    pub purple: Option<u8>,
    pub yellow: Option<u8>,
    pub dim_green: Option<u8>,
    pub dim_yellow: Option<u8>,
    pub dim_orange: Option<u8>,
    pub dim_red: Option<u8>,
    pub dim_cyan: Option<u8>,
    pub dim_pink: Option<u8>,
}

impl ThemeConfig {
    pub fn to_theme(&self) -> Theme {
        let c = |custom: Option<u8>, fallback: &'static str| -> String {
            match custom {
                Some(idx) => format!("\x1b[38;5;{idx}m"),
                None => fallback.into(),
            }
        };
        Theme {
            green: c(self.green, "\x1b[38;5;114m"),
            orange: c(self.orange, "\x1b[38;5;215m"),
            red: c(self.red, "\x1b[38;5;203m"),
            dim: c(self.dim, "\x1b[38;5;242m"),
            lgray: c(self.lgray, "\x1b[38;5;250m"),
            cyan: c(self.cyan, "\x1b[38;5;111m"),
            purple: c(self.purple, "\x1b[38;5;183m"),
            yellow: c(self.yellow, "\x1b[38;5;228m"),
            reset: "\x1b[0m".into(),
            dim_green: c(self.dim_green, "\x1b[38;5;65m"),
            dim_yellow: c(self.dim_yellow, "\x1b[38;5;136m"),
            dim_orange: c(self.dim_orange, "\x1b[38;5;130m"),
            dim_red: c(self.dim_red, "\x1b[38;5;131m"),
            dim_cyan: c(self.dim_cyan, "\x1b[38;5;67m"),
            dim_pink: c(self.dim_pink, "\x1b[38;5;175m"),
            italic: "\x1b[3m".into(),
            no_italic: "\x1b[23m".into(),
        }
    }
}

/// Customizable icon/symbol overrides.
#[derive(Debug, Clone, Default)]
pub struct IconsConfig {
    pub duration: Option<String>,
    pub cost: Option<String>,
    pub git_branch: Option<String>,
    pub git_staged: Option<String>,
    pub agent: Option<String>,
    pub update: Option<String>,
    pub bar_fill: Option<char>,
    pub bar_empty: Option<char>,
    pub bar_half: Option<char>,
    pub quota_fill: Option<char>,
    pub quota_empty: Option<char>,
    pub quota_pace: Option<char>,
}

/// Visual style for progress bars.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum BarStyle {
    #[default]
    Block,
    Dot,
    Ascii,
}

impl fmt::Display for BarStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block => write!(f, "block"),
            Self::Dot => write!(f, "dot"),
            Self::Ascii => write!(f, "ascii"),
        }
    }
}

impl FromStr for BarStyle {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dot" => Ok(Self::Dot),
            "ascii" => Ok(Self::Ascii),
            _ => Ok(Self::Block),
        }
    }
}

/// Convert ANSI escape sequences in a string to HTML spans.
///
/// Handles 256-color foreground (`\x1b[38;5;Nm`), italic (`\x1b[3m`),
/// end-italic (`\x1b[23m`), and reset (`\x1b[0m`).
/// Unrecognized ANSI sequences are stripped.
pub fn ansi_to_html(s: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    // Matches CSI sequences ending in 'm' (with capture group for code)
    // and OSC sequences (no capture group — these get stripped).
    static ANSI_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1b\[([^m]*)m|\x1b\][^\x07]*\x07").unwrap());

    let mut result = String::new();
    let mut open_spans: usize = 0;
    let mut last = 0;

    for cap in ANSI_RE.captures_iter(s) {
        let full = cap.get(0).unwrap();
        result.push_str(&s[last..full.start()]);
        last = full.end();

        // Only process CSI m sequences (group 1 present)
        if let Some(code_match) = cap.get(1) {
            let code = code_match.as_str();
            if code == "0" {
                for _ in 0..open_spans {
                    result.push_str("</span>");
                }
                open_spans = 0;
            } else if code == "3" {
                result.push_str("<span style=\"font-style:italic\">");
                open_spans += 1;
            } else if code == "23" {
                if open_spans > 0 {
                    result.push_str("</span>");
                    open_spans -= 1;
                }
            } else if let Some(n) = code.strip_prefix("38;5;") {
                if let Ok(idx) = n.parse::<u8>() {
                    let hex = ansi_256_to_hex(idx);
                    result.push_str(&format!("<span style=\"color:{hex}\">"));
                    open_spans += 1;
                }
            }
            // else: unrecognized CSI code — stripped
        }
        // else: OSC or other sequence — stripped
    }

    result.push_str(&s[last..]);
    for _ in 0..open_spans {
        result.push_str("</span>");
    }

    result
}

/// Convert a 256-color index to an approximate hex string (#RRGGBB).
pub fn ansi_256_to_hex(idx: u8) -> String {
    let (r, g, b) = match idx {
        // Standard colors (0-7)
        0 => (0u8, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        // High-intensity colors (8-15)
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // 6x6x6 color cube (16-231)
        16..=231 => {
            let n = idx - 16;
            let ri = n / 36;
            let gi = (n % 36) / 6;
            let bi = n % 6;
            let to_val = |i: u8| if i == 0 { 0u8 } else { 55 + 40 * i };
            (to_val(ri), to_val(gi), to_val(bi))
        }
        // Grayscale ramp (232-255)
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    };
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors;

    #[test]
    fn default_theme_matches_colors() {
        let t = Theme::default();
        assert_eq!(t.green, colors::GREEN);
        assert_eq!(t.orange, colors::ORANGE);
        assert_eq!(t.red, colors::RED);
        assert_eq!(t.dim, colors::DIM);
        assert_eq!(t.lgray, colors::LGRAY);
        assert_eq!(t.cyan, colors::CYAN);
        assert_eq!(t.purple, colors::PURPLE);
        assert_eq!(t.yellow, colors::YELLOW);
        assert_eq!(t.reset, colors::RESET);
        assert_eq!(t.dim_green, colors::DIM_GREEN);
        assert_eq!(t.dim_yellow, colors::DIM_YELLOW);
        assert_eq!(t.dim_orange, colors::DIM_ORANGE);
        assert_eq!(t.dim_red, colors::DIM_RED);
        assert_eq!(t.dim_cyan, colors::DIM_CYAN);
        assert_eq!(t.dim_pink, colors::DIM_PINK);
        assert_eq!(t.italic, colors::ITALIC);
        assert_eq!(t.no_italic, colors::NO_ITALIC);
    }

    #[test]
    fn custom_theme_single_override() {
        let cfg = ThemeConfig {
            green: Some(46),
            ..ThemeConfig::default()
        };
        let t = cfg.to_theme();
        assert_eq!(t.green, "\x1b[38;5;46m");
        // All others should match defaults
        let d = Theme::default();
        assert_eq!(t.orange, d.orange);
        assert_eq!(t.red, d.red);
        assert_eq!(t.dim, d.dim);
        assert_eq!(t.reset, d.reset);
        assert_eq!(t.italic, d.italic);
        assert_eq!(t.no_italic, d.no_italic);
    }

    #[test]
    fn theme_config_default_produces_default_theme() {
        assert_eq!(ThemeConfig::default().to_theme(), Theme::default());
    }

    #[test]
    fn ansi_256_to_hex_known_values() {
        // The 10 indices used in colors.rs
        assert_eq!(ansi_256_to_hex(114), "#87d787"); // GREEN
        assert_eq!(ansi_256_to_hex(215), "#ffaf5f"); // ORANGE
        assert_eq!(ansi_256_to_hex(203), "#ff5f5f"); // RED
        assert_eq!(ansi_256_to_hex(242), "#6c6c6c"); // DIM
        assert_eq!(ansi_256_to_hex(250), "#bcbcbc"); // LGRAY
        assert_eq!(ansi_256_to_hex(111), "#87afff"); // CYAN
        assert_eq!(ansi_256_to_hex(183), "#d7afff"); // PURPLE
        assert_eq!(ansi_256_to_hex(228), "#ffff87"); // YELLOW
        assert_eq!(ansi_256_to_hex(65), "#5f875f"); // DIM_GREEN
        assert_eq!(ansi_256_to_hex(136), "#af8700"); // DIM_YELLOW
        assert_eq!(ansi_256_to_hex(130), "#af5f00"); // DIM_ORANGE
        assert_eq!(ansi_256_to_hex(131), "#af5f5f"); // DIM_RED
        assert_eq!(ansi_256_to_hex(67), "#5f87af"); // DIM_CYAN
        assert_eq!(ansi_256_to_hex(175), "#d787af"); // DIM_PINK
    }

    #[test]
    fn bar_style_display() {
        assert_eq!(BarStyle::Block.to_string(), "block");
        assert_eq!(BarStyle::Dot.to_string(), "dot");
        assert_eq!(BarStyle::Ascii.to_string(), "ascii");
    }

    #[test]
    fn bar_style_default_is_block() {
        assert_eq!(BarStyle::default(), BarStyle::Block);
    }

    #[test]
    fn ansi_to_html_basic() {
        let input = "\x1b[38;5;114mhello\x1b[0m";
        assert_eq!(
            ansi_to_html(input),
            "<span style=\"color:#87d787\">hello</span>"
        );
    }

    #[test]
    fn ansi_to_html_nested_colors() {
        let input = "\x1b[38;5;114mgreen \x1b[3mitalic\x1b[23m\x1b[0m";
        assert_eq!(
            ansi_to_html(input),
            "<span style=\"color:#87d787\">green <span style=\"font-style:italic\">italic</span></span>"
        );
    }

    #[test]
    fn ansi_to_html_strips_unknown() {
        // Bold (\x1b[1m) is not recognized — should be stripped
        let input = "\x1b[1mbold\x1b[0m";
        assert_eq!(ansi_to_html(input), "bold");
    }
}
