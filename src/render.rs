use anyhow::Result;

use crate::config::StatuslineConfig;
use crate::fmt::visible_len;
use crate::types::StdinData;
use crate::widgets;

fn read_stdin_nonblocking() -> StdinData {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        return StdinData::default();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let data = serde_json::from_reader(std::io::stdin()).unwrap_or_default();
        let _ = tx.send(data);
    });
    rx.recv_timeout(std::time::Duration::from_millis(200))
        .unwrap_or_default()
}

pub fn join_segments(segments: &[String], separator: &str, max_width: u16) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let sep_w = visible_len(separator);
    let max_w = max_width as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut cur = segments[0].clone();
    let mut cur_w = visible_len(&cur);
    for seg in &segments[1..] {
        let seg_w = visible_len(seg);
        if cur_w + sep_w + seg_w <= max_w {
            cur.push_str(separator);
            cur.push_str(seg);
            cur_w += sep_w + seg_w;
        } else {
            lines.push(cur);
            cur = seg.clone();
            cur_w = seg_w;
        }
    }
    lines.push(cur);
    lines.join("\n")
}

pub fn run() -> Result<()> {
    let config = StatuslineConfig::load()?;
    let data = read_stdin_nonblocking();
    let ctx = widgets::build_context(data, &config);
    let sep = &config.separator;
    let width = ctx.terminal_width;

    let lines = [&config.line1, &config.line2, &config.line3];
    let mut output_lines: Vec<String> = Vec::new();

    for line_widgets in lines.iter() {
        let parts = widgets::render_line_parts(line_widgets, &ctx, &config.widgets);
        if parts.is_empty() {
            continue;
        }
        let joined = join_segments(&parts, sep, width);
        output_lines.push(joined);
    }

    if !output_lines.is_empty() {
        print!("{}", output_lines.join("\n"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_empty() {
        assert_eq!(join_segments(&[], " | ", 80), "");
    }

    #[test]
    fn join_single() {
        assert_eq!(join_segments(&["hello".to_string()], " | ", 80), "hello");
    }

    #[test]
    fn join_fits_on_one_line() {
        let segs = vec!["aaa".to_string(), "bbb".to_string()];
        assert_eq!(join_segments(&segs, " | ", 80), "aaa | bbb");
    }

    #[test]
    fn join_wraps_when_too_wide() {
        let segs = vec!["aaaa".to_string(), "bbbb".to_string()];
        // max_width=8, sep=" | " (3), 4+3+4=11 > 8 → wraps
        let result = join_segments(&segs, " | ", 8);
        assert_eq!(result, "aaaa\nbbbb");
    }

    #[test]
    fn join_three_segments_partial_wrap() {
        let segs = vec!["aa".to_string(), "bb".to_string(), "cc".to_string()];
        // sep="|" (1), max_width=6
        // "aa|bb" = 5 fits; "aa|bb|cc" = 8 > 6 → wrap cc
        let result = join_segments(&segs, "|", 6);
        assert_eq!(result, "aa|bb\ncc");
    }
}
