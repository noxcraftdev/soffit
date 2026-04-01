use crate::theme::PaletteRole;

#[allow(dead_code)]
pub struct ColorSlot {
    pub key: &'static str,
    pub label: &'static str,
    pub theme_field: &'static str,
    pub default_idx: u8,
    pub default_role: Option<PaletteRole>,
}

#[allow(dead_code)]
pub struct IconSlot {
    pub key: &'static str,
    pub label: &'static str,
    pub icons_field: &'static str,
    pub default_value: &'static str,
    pub is_char: bool,
}

pub struct WidgetRef {
    pub name: &'static str,
    pub description: &'static str,
    pub default_components: &'static [&'static str],
    pub has_compact: bool,
    #[allow(dead_code)]
    pub color_slots: &'static [ColorSlot],
    #[allow(dead_code)]
    pub icon_slots: &'static [IconSlot],
}

pub const WIDGETS: &[WidgetRef] = &[
    WidgetRef {
        name: "version",
        description: "Claude Code version + model name",
        default_components: &["update", "version", "model"],
        has_compact: true,
        color_slots: &[
            ColorSlot {
                key: "update_indicator",
                label: "Update Arrow",
                theme_field: "orange",
                default_idx: 215,
                default_role: Some(PaletteRole::Warning),
            },
            ColorSlot {
                key: "version_text",
                label: "Version",
                theme_field: "dim",
                default_idx: 242,
                default_role: Some(PaletteRole::Muted),
            },
            ColorSlot {
                key: "model_name",
                label: "Model",
                theme_field: "purple",
                default_idx: 183,
                default_role: Some(PaletteRole::Accent),
            },
        ],
        icon_slots: &[IconSlot {
            key: "update",
            label: "Update Icon",
            icons_field: "update",
            default_value: "↑ ",
            is_char: false,
        }],
    },
    WidgetRef {
        name: "context_bar",
        description: "Context window usage bar",
        default_components: &["bar", "pct", "tokens"],
        has_compact: true,
        color_slots: &[
            ColorSlot {
                key: "low",
                label: "Low Usage",
                theme_field: "green",
                default_idx: 114,
                default_role: Some(PaletteRole::Success),
            },
            ColorSlot {
                key: "mid",
                label: "Mid Usage",
                theme_field: "orange",
                default_idx: 215,
                default_role: Some(PaletteRole::Warning),
            },
            ColorSlot {
                key: "high",
                label: "High Usage",
                theme_field: "red",
                default_idx: 203,
                default_role: Some(PaletteRole::Danger),
            },
            ColorSlot {
                key: "empty",
                label: "Empty Bar",
                theme_field: "dim",
                default_idx: 242,
                default_role: Some(PaletteRole::Muted),
            },
        ],
        icon_slots: &[
            IconSlot {
                key: "bar_fill",
                label: "Bar Fill",
                icons_field: "bar_fill",
                default_value: "■",
                is_char: true,
            },
            IconSlot {
                key: "bar_empty",
                label: "Bar Empty",
                icons_field: "bar_empty",
                default_value: "□",
                is_char: true,
            },
            IconSlot {
                key: "bar_half",
                label: "Bar Half",
                icons_field: "bar_half",
                default_value: "◧",
                is_char: true,
            },
        ],
    },
    WidgetRef {
        name: "duration",
        description: "Session duration",
        default_components: &[],
        has_compact: false,
        color_slots: &[ColorSlot {
            key: "time",
            label: "Duration",
            theme_field: "lgray",
            default_idx: 250,
            default_role: Some(PaletteRole::Subtle),
        }],
        icon_slots: &[IconSlot {
            key: "duration",
            label: "Duration Icon",
            icons_field: "duration",
            default_value: "⏱ ",
            is_char: false,
        }],
    },
    WidgetRef {
        name: "cost",
        description: "Session and daily cost",
        default_components: &["session", "today", "week"],
        has_compact: true,
        color_slots: &[
            ColorSlot {
                key: "within_budget",
                label: "Within Budget",
                theme_field: "green",
                default_idx: 114,
                default_role: Some(PaletteRole::Success),
            },
            ColorSlot {
                key: "approaching",
                label: "Approaching",
                theme_field: "orange",
                default_idx: 215,
                default_role: Some(PaletteRole::Warning),
            },
            ColorSlot {
                key: "over_budget",
                label: "Over Budget",
                theme_field: "red",
                default_idx: 203,
                default_role: Some(PaletteRole::Danger),
            },
        ],
        icon_slots: &[IconSlot {
            key: "cost",
            label: "Cost Icon",
            icons_field: "cost",
            default_value: "💸 ",
            is_char: false,
        }],
    },
    WidgetRef {
        name: "git",
        description: "Branch, staged/modified counts, repo link",
        default_components: &["branch", "staged", "modified", "repo", "worktree"],
        has_compact: true,
        color_slots: &[
            ColorSlot {
                key: "branch",
                label: "Branch",
                theme_field: "lgray",
                default_idx: 250,
                default_role: Some(PaletteRole::Subtle),
            },
            ColorSlot {
                key: "staged",
                label: "Staged",
                theme_field: "green",
                default_idx: 114,
                default_role: Some(PaletteRole::Success),
            },
            ColorSlot {
                key: "modified",
                label: "Modified",
                theme_field: "orange",
                default_idx: 215,
                default_role: Some(PaletteRole::Warning),
            },
            ColorSlot {
                key: "repo",
                label: "Repository",
                theme_field: "cyan",
                default_idx: 111,
                default_role: Some(PaletteRole::Primary),
            },
            ColorSlot {
                key: "worktree",
                label: "Worktree",
                theme_field: "dim_pink",
                default_idx: 175,
                default_role: Some(PaletteRole::Accent),
            },
        ],
        icon_slots: &[
            IconSlot {
                key: "branch",
                label: "Branch Icon",
                icons_field: "git_branch",
                default_value: "⎇ ",
                is_char: false,
            },
            IconSlot {
                key: "staged",
                label: "Staged Icon",
                icons_field: "git_staged",
                default_value: "•",
                is_char: false,
            },
        ],
    },
    WidgetRef {
        name: "quota",
        description: "Rate limit usage bars with pace tracking",
        default_components: &["five_hour", "seven_day"],
        has_compact: true,
        color_slots: &[
            ColorSlot {
                key: "healthy",
                label: "Healthy",
                theme_field: "cyan",
                default_idx: 111,
                default_role: Some(PaletteRole::Primary),
            },
            ColorSlot {
                key: "warning",
                label: "Warning",
                theme_field: "orange",
                default_idx: 215,
                default_role: Some(PaletteRole::Warning),
            },
            ColorSlot {
                key: "critical",
                label: "Critical",
                theme_field: "red",
                default_idx: 203,
                default_role: Some(PaletteRole::Danger),
            },
        ],
        icon_slots: &[
            IconSlot {
                key: "quota_fill",
                label: "Fill Char",
                icons_field: "quota_fill",
                default_value: "●",
                is_char: true,
            },
            IconSlot {
                key: "quota_empty",
                label: "Empty Char",
                icons_field: "quota_empty",
                default_value: "○",
                is_char: true,
            },
            IconSlot {
                key: "quota_pace",
                label: "Pace Char",
                icons_field: "quota_pace",
                default_value: "◌",
                is_char: true,
            },
        ],
    },
    WidgetRef {
        name: "vim",
        description: "Current vim mode",
        default_components: &[],
        has_compact: false,
        color_slots: &[ColorSlot {
            key: "mode",
            label: "Vim Mode",
            theme_field: "purple",
            default_idx: 183,
            default_role: Some(PaletteRole::Accent),
        }],
        icon_slots: &[],
    },
    WidgetRef {
        name: "agent",
        description: "Active agent name",
        default_components: &[],
        has_compact: false,
        color_slots: &[ColorSlot {
            key: "name",
            label: "Agent Name",
            theme_field: "orange",
            default_idx: 215,
            default_role: Some(PaletteRole::Warning),
        }],
        icon_slots: &[IconSlot {
            key: "agent",
            label: "Agent Icon",
            icons_field: "agent",
            default_value: "❯ ",
            is_char: false,
        }],
    },
    WidgetRef {
        name: "session",
        description: "Shortest unique session ID prefix",
        default_components: &[],
        has_compact: false,
        color_slots: &[ColorSlot {
            key: "id",
            label: "Session ID",
            theme_field: "dim",
            default_idx: 242,
            default_role: Some(PaletteRole::Muted),
        }],
        icon_slots: &[],
    },
];

pub fn widget_ref(name: &str) -> Option<&'static WidgetRef> {
    WIDGETS.iter().find(|w| w.name == name)
}

pub fn component_desc(widget: &str, comp: &str) -> &'static str {
    match (widget, comp) {
        ("version", "update") => "↑ arrow when update available",
        ("version", "version") => "version number",
        ("version", "model") => "model name",
        ("context_bar", "bar") => "visual fill bar ▓▓▓░░░",
        ("context_bar", "pct") => "usage %",
        ("context_bar", "tokens") => "tokens used / limit (hidden in compact)",
        ("cost", "session") => "this session",
        ("cost", "today") => "today's total",
        ("cost", "week") => "this week",
        ("git", "branch") => "current branch",
        ("git", "staged") => "staged file count (hidden in compact)",
        ("git", "modified") => "modified file count (hidden in compact)",
        ("git", "repo") => "repo name (hidden in compact)",
        ("git", "worktree") => "worktree name (hidden in compact)",
        ("quota", "five_hour") => "5-hour rate limit bar",
        ("quota", "seven_day") => "7-day rate limit bar",
        ("insights", "strategies") => "active strategies (purple)",
        ("insights", "priorities") => "priority items (red)",
        ("insights", "insights") => "insights (orange)",
        ("insights", "notes") => "notes (green)",
        ("insights", "pending") => "pending actions (yellow)",
        _ => "",
    }
}
