pub struct WidgetRef {
    pub name: &'static str,
    pub description: &'static str,
    pub default_components: &'static [&'static str],
    pub has_compact: bool,
}

pub const WIDGETS: &[WidgetRef] = &[
    WidgetRef {
        name: "version",
        description: "Claude Code version + model name",
        default_components: &["update", "version", "model"],
        has_compact: true,
    },
    WidgetRef {
        name: "context_bar",
        description: "Context window usage bar",
        default_components: &["bar", "pct", "tokens"],
        has_compact: true,
    },
    WidgetRef {
        name: "duration",
        description: "Session duration",
        default_components: &[],
        has_compact: false,
    },
    WidgetRef {
        name: "cost",
        description: "Session and daily cost",
        default_components: &["session", "today", "week"],
        has_compact: true,
    },
    WidgetRef {
        name: "git",
        description: "Branch, staged/modified counts, repo link",
        default_components: &["branch", "staged", "modified", "repo", "worktree"],
        has_compact: true,
    },
    WidgetRef {
        name: "quota",
        description: "Rate limit usage bars with pace tracking",
        default_components: &["five_hour", "seven_day"],
        has_compact: true,
    },
    WidgetRef {
        name: "vim",
        description: "Current vim mode",
        default_components: &[],
        has_compact: false,
    },
    WidgetRef {
        name: "agent",
        description: "Active agent name",
        default_components: &[],
        has_compact: false,
    },
    WidgetRef {
        name: "session",
        description: "Shortest unique session ID prefix",
        default_components: &[],
        has_compact: false,
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
