use crate::theme::PaletteRole;

pub struct ThemeSlot {
    pub key: &'static str,
    pub label: &'static str,
    pub palette_role: PaletteRole,
    #[allow(dead_code)]
    pub default_idx: u8,
}

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
    pub color_slots: &'static [ThemeSlot],
    pub icon_slots: &'static [IconSlot],
}

pub const WIDGETS: &[WidgetRef] = &[
    WidgetRef {
        name: "version",
        description: "Claude Code version + model name",
        default_components: &["update", "version", "model"],
        has_compact: true,
        color_slots: &[
            ThemeSlot {
                key: "update_indicator",
                label: "Update Arrow",
                palette_role: PaletteRole::Warning,
                default_idx: 215,
            },
            ThemeSlot {
                key: "version_text",
                label: "Version",
                palette_role: PaletteRole::Muted,
                default_idx: 242,
            },
            ThemeSlot {
                key: "model_name",
                label: "Model",
                palette_role: PaletteRole::Accent,
                default_idx: 183,
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
            ThemeSlot {
                key: "low",
                label: "Low Usage",
                palette_role: PaletteRole::Success,
                default_idx: 114,
            },
            ThemeSlot {
                key: "mid",
                label: "Mid Usage",
                palette_role: PaletteRole::Warning,
                default_idx: 215,
            },
            ThemeSlot {
                key: "high",
                label: "High Usage",
                palette_role: PaletteRole::Danger,
                default_idx: 203,
            },
            ThemeSlot {
                key: "empty",
                label: "Empty Bar",
                palette_role: PaletteRole::Muted,
                default_idx: 242,
            },
        ],
        icon_slots: &[],
    },
    WidgetRef {
        name: "duration",
        description: "Session duration",
        default_components: &[],
        has_compact: false,
        color_slots: &[
            ThemeSlot {
                key: "time",
                label: "< 30m",
                palette_role: PaletteRole::Subtle,
                default_idx: 250,
            },
            ThemeSlot {
                key: "time_medium",
                label: "30m\u{2013}1h",
                palette_role: PaletteRole::Muted,
                default_idx: 242,
            },
            ThemeSlot {
                key: "time_high",
                label: "1h\u{2013}2h",
                palette_role: PaletteRole::Warning,
                default_idx: 208,
            },
            ThemeSlot {
                key: "time_critical",
                label: "> 2h",
                palette_role: PaletteRole::Danger,
                default_idx: 196,
            },
        ],
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
            ThemeSlot {
                key: "within_budget",
                label: "Within Budget",
                palette_role: PaletteRole::Success,
                default_idx: 114,
            },
            ThemeSlot {
                key: "approaching",
                label: "Approaching",
                palette_role: PaletteRole::Warning,
                default_idx: 215,
            },
            ThemeSlot {
                key: "over_budget",
                label: "Over Budget",
                palette_role: PaletteRole::Danger,
                default_idx: 203,
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
            ThemeSlot {
                key: "branch",
                label: "Branch",
                palette_role: PaletteRole::Subtle,
                default_idx: 250,
            },
            ThemeSlot {
                key: "staged",
                label: "Staged",
                palette_role: PaletteRole::Success,
                default_idx: 114,
            },
            ThemeSlot {
                key: "modified",
                label: "Modified",
                palette_role: PaletteRole::Warning,
                default_idx: 215,
            },
            ThemeSlot {
                key: "repo",
                label: "Repository",
                palette_role: PaletteRole::Primary,
                default_idx: 111,
            },
            ThemeSlot {
                key: "worktree",
                label: "Worktree",
                palette_role: PaletteRole::Accent,
                default_idx: 175,
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
            ThemeSlot {
                key: "healthy",
                label: "Healthy",
                palette_role: PaletteRole::Primary,
                default_idx: 111,
            },
            ThemeSlot {
                key: "warning",
                label: "Warning",
                palette_role: PaletteRole::Warning,
                default_idx: 215,
            },
            ThemeSlot {
                key: "critical",
                label: "Critical",
                palette_role: PaletteRole::Danger,
                default_idx: 203,
            },
        ],
        icon_slots: &[],
    },
    WidgetRef {
        name: "vim",
        description: "Current vim mode",
        default_components: &[],
        has_compact: false,
        color_slots: &[ThemeSlot {
            key: "mode",
            label: "Vim Mode",
            palette_role: PaletteRole::Accent,
            default_idx: 183,
        }],
        icon_slots: &[],
    },
    WidgetRef {
        name: "agent",
        description: "Active agent name",
        default_components: &[],
        has_compact: false,
        color_slots: &[ThemeSlot {
            key: "name",
            label: "Agent Name",
            palette_role: PaletteRole::Warning,
            default_idx: 215,
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
        color_slots: &[ThemeSlot {
            key: "id",
            label: "Session ID",
            palette_role: PaletteRole::Muted,
            default_idx: 242,
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
