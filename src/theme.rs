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
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
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
    #[allow(dead_code)]
    pub fn get_field(&self, name: &str) -> Option<u8> {
        match name {
            "green" => self.green,
            "orange" => self.orange,
            "red" => self.red,
            "dim" => self.dim,
            "lgray" => self.lgray,
            "cyan" => self.cyan,
            "purple" => self.purple,
            "yellow" => self.yellow,
            "dim_green" => self.dim_green,
            "dim_yellow" => self.dim_yellow,
            "dim_orange" => self.dim_orange,
            "dim_red" => self.dim_red,
            "dim_cyan" => self.dim_cyan,
            "dim_pink" => self.dim_pink,
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn set_field(&mut self, name: &str, value: Option<u8>) {
        match name {
            "green" => self.green = value,
            "orange" => self.orange = value,
            "red" => self.red = value,
            "dim" => self.dim = value,
            "lgray" => self.lgray = value,
            "cyan" => self.cyan = value,
            "purple" => self.purple = value,
            "yellow" => self.yellow = value,
            "dim_green" => self.dim_green = value,
            "dim_yellow" => self.dim_yellow = value,
            "dim_orange" => self.dim_orange = value,
            "dim_red" => self.dim_red = value,
            "dim_cyan" => self.dim_cyan = value,
            "dim_pink" => self.dim_pink = value,
            _ => {}
        }
    }

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

/// Semantic roles for a theme palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PaletteRole {
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Muted,
    Subtle,
}

#[allow(dead_code)]
pub const PALETTE_ROLES: &[PaletteRole] = &[
    PaletteRole::Primary,
    PaletteRole::Accent,
    PaletteRole::Success,
    PaletteRole::Warning,
    PaletteRole::Danger,
    PaletteRole::Muted,
    PaletteRole::Subtle,
];

#[allow(dead_code)]
impl PaletteRole {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Accent => "accent",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Muted => "muted",
            Self::Subtle => "subtle",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Accent => "Accent",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Danger => "Danger",
            Self::Muted => "Muted",
            Self::Subtle => "Subtle",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "primary" => Some(Self::Primary),
            "accent" => Some(Self::Accent),
            "success" => Some(Self::Success),
            "warning" => Some(Self::Warning),
            "danger" => Some(Self::Danger),
            "muted" => Some(Self::Muted),
            "subtle" => Some(Self::Subtle),
            _ => None,
        }
    }
}

/// A named set of 7 semantic color indices (ANSI 256).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThemePalette {
    pub primary: u8,
    pub accent: u8,
    pub success: u8,
    pub warning: u8,
    pub danger: u8,
    pub muted: u8,
    pub subtle: u8,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            primary: 111,
            accent: 183,
            success: 114,
            warning: 215,
            danger: 203,
            muted: 242,
            subtle: 250,
        }
    }
}

#[allow(dead_code)]
impl ThemePalette {
    pub fn resolve(&self, role: PaletteRole) -> u8 {
        match role {
            PaletteRole::Primary => self.primary,
            PaletteRole::Accent => self.accent,
            PaletteRole::Success => self.success,
            PaletteRole::Warning => self.warning,
            PaletteRole::Danger => self.danger,
            PaletteRole::Muted => self.muted,
            PaletteRole::Subtle => self.subtle,
        }
    }
}

#[allow(dead_code)]
pub const THEME_PRESETS: &[(&str, ThemePalette)] = &[
    (
        "Default",
        ThemePalette {
            primary: 111,
            accent: 183,
            success: 114,
            warning: 215,
            danger: 203,
            muted: 242,
            subtle: 250,
        },
    ),
    (
        "Nord",
        ThemePalette {
            primary: 111,
            accent: 147,
            success: 114,
            warning: 222,
            danger: 210,
            muted: 243,
            subtle: 252,
        },
    ),
    (
        "Warm",
        ThemePalette {
            primary: 214,
            accent: 211,
            success: 150,
            warning: 221,
            danger: 196,
            muted: 240,
            subtle: 248,
        },
    ),
    (
        "Mono",
        ThemePalette {
            primary: 252,
            accent: 248,
            success: 250,
            warning: 246,
            danger: 244,
            muted: 240,
            subtle: 236,
        },
    ),
    (
        "Dracula",
        ThemePalette {
            primary: 141,
            accent: 212,
            success: 84,
            warning: 228,
            danger: 210,
            muted: 61,
            subtle: 189,
        },
    ),
];

/// Curated ANSI 256-color indices for color pickers (skip 0-15 terminal-dependent).
#[allow(dead_code)]
pub const CURATED_COLORS: &[u8] = &[
    // Blues
    21, 27, 33, 39, 63, 69, 75, 111, // Cyans/teals
    44, 45, 50, 51, 80, 81, 86, 87, // Greens
    46, 82, 83, 84, 112, 113, 114, 118, 119, 150, // Yellows/oranges
    136, 178, 214, 215, 220, 221, 222, 226, 227, 228, // Reds/pinks
    196, 197, 198, 203, 204, 210, 211, 212, // Purples/magentas
    129, 135, 141, 147, 165, 171, 177, 183, 189, // Muted/dim
    61, 65, 67, 95, 96, 100, 101, 130, 131, 175, // Grayscale
    232, 234, 236, 238, 240, 242, 244, 246, 248, 250, 252, 254, 255,
];

/// Customizable icon/symbol overrides.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
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

impl IconsConfig {
    #[allow(dead_code)]
    pub fn get_string_field(&self, name: &str) -> Option<&str> {
        match name {
            "duration" => self.duration.as_deref(),
            "cost" => self.cost.as_deref(),
            "git_branch" => self.git_branch.as_deref(),
            "git_staged" => self.git_staged.as_deref(),
            "agent" => self.agent.as_deref(),
            "update" => self.update.as_deref(),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_char_field(&self, name: &str) -> Option<char> {
        match name {
            "bar_fill" => self.bar_fill,
            "bar_empty" => self.bar_empty,
            "bar_half" => self.bar_half,
            "quota_fill" => self.quota_fill,
            "quota_empty" => self.quota_empty,
            "quota_pace" => self.quota_pace,
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn set_string_field(&mut self, name: &str, value: Option<String>) {
        match name {
            "duration" => self.duration = value,
            "cost" => self.cost = value,
            "git_branch" => self.git_branch = value,
            "git_staged" => self.git_staged = value,
            "agent" => self.agent = value,
            "update" => self.update = value,
            _ => {}
        }
    }

    #[allow(dead_code)]
    pub fn set_char_field(&mut self, name: &str, value: Option<char>) {
        match name {
            "bar_fill" => self.bar_fill = value,
            "bar_empty" => self.bar_empty = value,
            "bar_half" => self.bar_half = value,
            "quota_fill" => self.quota_fill = value,
            "quota_empty" => self.quota_empty = value,
            "quota_pace" => self.quota_pace = value,
            _ => {}
        }
    }
}

/// Visual style for progress bars.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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

    #[test]
    fn theme_config_get_set_field() {
        let mut tc = ThemeConfig::default();
        assert_eq!(tc.get_field("green"), None);
        tc.set_field("green", Some(46));
        assert_eq!(tc.get_field("green"), Some(46));
        assert_eq!(tc.green, Some(46));
        tc.set_field("green", None);
        assert_eq!(tc.get_field("green"), None);
        // Unknown field is a no-op
        tc.set_field("nonexistent", Some(99));
        assert_eq!(tc.get_field("nonexistent"), None);
    }

    #[test]
    fn icons_config_get_set_fields() {
        let mut ic = IconsConfig::default();
        assert_eq!(ic.get_string_field("cost"), None);
        ic.set_string_field("cost", Some("$$$".into()));
        assert_eq!(ic.get_string_field("cost"), Some("$$$"));
        assert_eq!(ic.get_char_field("bar_fill"), None);
        ic.set_char_field("bar_fill", Some('X'));
        assert_eq!(ic.get_char_field("bar_fill"), Some('X'));
    }
}
