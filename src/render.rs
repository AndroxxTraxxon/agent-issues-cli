use std::io::IsTerminal;

use anstyle::{AnsiColor, Color, Effects, Reset, Style};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Whether the human view should emit ANSI escapes: only on a terminal, and only
/// when NO_COLOR is unset. The plain output stays byte-stable when piped.
pub fn should_colorize() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn status_style(status: &str) -> Style {
    match status {
        "open" => Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen))),
        "in-progress" => Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightYellow))),
        "blocked" => Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightRed))),
        "closed" => Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack))),
        _ => Style::new(),
    }
}

fn label_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightCyan)))
}

fn link_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::BrightBlue)))
        .underline()
}

fn dim_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}

fn code_block_style() -> Style {
    if is_dark_background() {
        dim_style()
    } else {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Black)))
    }
}

fn is_dark_background() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|v| v.split(';').nth(1).and_then(|s| s.parse::<u8>().ok()))
        .map(|bg| bg < 8)
        .unwrap_or(true)
}

/// Style + text + reset. Padding is the caller's job so escape codes never
/// throw off column alignment.
pub fn paint(style: &Style, text: &str) -> String {
    format!("{}{}{}", style.render(), text, style.render_reset())
}

pub fn paint_title(text: &str) -> String {
    paint(&Style::new().bold(), text)
}

pub fn paint_status(text: &str) -> String {
    paint(&status_style(text), text)
}

pub fn paint_label(text: &str) -> String {
    paint(&label_style(), text)
}

/// Render a markdown body as ANSI-styled terminal text. Callers gate this on
/// `should_colorize()`; when the flag is off, use the raw body instead.
pub fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);

    let mut out = String::new();
    let mut code_buf = String::new();
    let mut in_code_block = false;
    let mut list_depth = 0usize;

    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_table = false;
    let mut in_cell = false;

    for ev in parser {
        if in_table {
            match ev {
                Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                    table_row = Vec::new();
                }
                Event::Start(Tag::TableCell) => {
                    in_cell = true;
                    cell = String::new();
                }
                Event::End(TagEnd::TableCell) => {
                    in_cell = false;
                    table_row.push(cell.clone());
                }
                Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                    table_rows.push(std::mem::take(&mut table_row));
                }
                Event::End(TagEnd::Table) => {
                    in_table = false;
                    out.push_str(&render_table(&table_rows));
                }
                Event::Start(Tag::Emphasis) => {
                    cell.push_str(&Style::new().italic().render().to_string())
                }
                Event::Start(Tag::Strong) => {
                    cell.push_str(&Style::new().bold().render().to_string())
                }
                Event::Start(Tag::Strikethrough) => {
                    cell.push_str(&Style::new().strikethrough().render().to_string())
                }
                Event::Start(Tag::Link { .. }) => cell.push_str(&link_style().render().to_string()),
                Event::End(TagEnd::Emphasis)
                | Event::End(TagEnd::Strong)
                | Event::End(TagEnd::Strikethrough)
                | Event::End(TagEnd::Link) => cell.push_str(&Reset.render().to_string()),
                Event::Text(t) => cell.push_str(&t),
                Event::Code(t) => {
                    let s = Style::new().effects(Effects::INVERT);
                    cell.push_str(&format!("{}{}{}", s, t, s.render_reset()));
                }
                Event::SoftBreak | Event::HardBreak => cell.push(' '),
                _ => {}
            }
            continue;
        }

        match ev {
            Event::Start(tag) => match tag {
                Tag::Heading { .. } => out.push_str(&Style::new().bold().render().to_string()),
                Tag::Emphasis => out.push_str(&Style::new().italic().render().to_string()),
                Tag::Strong => out.push_str(&Style::new().bold().render().to_string()),
                Tag::Strikethrough => {
                    out.push_str(&Style::new().strikethrough().render().to_string())
                }
                Tag::Link { .. } => out.push_str(&link_style().render().to_string()),
                Tag::BlockQuote(_) => {
                    out.push_str(&dim_style().render().to_string());
                    out.push_str("> ");
                }
                Tag::List(_) => list_depth += 1,
                Tag::Item => {
                    out.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                    out.push_str("- ");
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    code_buf.clear();
                }
                Tag::Table(_) => {
                    in_table = true;
                    table_rows.clear();
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(..) | TagEnd::Item | TagEnd::Paragraph | TagEnd::BlockQuote(_) => {
                    push_line(&mut out);
                    out.push_str(&Reset.render().to_string());
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    out.push_str(&Reset.render().to_string())
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    out.push_str(&render_code_block(&code_buf));
                }
                TagEnd::List(_) => list_depth = list_depth.saturating_sub(1),
                _ => {}
            },
            Event::Text(t) => {
                if in_code_block {
                    code_buf.push_str(&t);
                } else {
                    out.push_str(&t);
                }
            }
            Event::Code(t) => {
                let s = Style::new().effects(Effects::INVERT);
                out.push_str(&format!("{}{}{}", s, t, s.render_reset()));
            }
            Event::TaskListMarker(checked) => out.push_str(if checked { "[x] " } else { "[ ] " }),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => {
                out.push_str(&dim_style().render().to_string());
                out.push_str("----------\n");
                out.push_str(&Reset.render().to_string());
            }
            _ => {}
        }
    }

    if in_cell {
        table_row.push(cell.clone());
    }
    if !table_row.is_empty() {
        table_rows.push(table_row);
    }

    out
}

fn push_line(out: &mut String) {
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

fn render_code_block(code: &str) -> String {
    let style = code_block_style();
    let body = code.strip_suffix('\n').unwrap_or(code);
    format!("{}{}{}\n", style.render(), body, Reset.render())
}

fn render_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let widths: Vec<usize> = (0..cols)
        .map(|c| {
            rows.iter()
                .filter_map(|r| r.get(c))
                .map(|s| plain_len(s))
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        for c in 0..cols {
            let cell = row.get(c).cloned().unwrap_or_default();
            let pad = widths[c].saturating_sub(plain_len(&cell));
            out.push_str(&cell);
            out.push_str(&" ".repeat(pad));
            if c + 1 < cols {
                out.push_str("  ");
            }
        }
        out.push('\n');
        if i == 0 {
            for w in &widths {
                out.push_str(&"-".repeat(*w));
                out.push_str("  ");
            }
            out.push('\n');
        }
    }
    out
}

fn plain_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_esc = false;
    for c in s.chars() {
        if in_esc {
            if c == 'm' {
                in_esc = false;
            }
        } else if c == '\x1b' {
            in_esc = true;
        } else {
            len += 1;
        }
    }
    len
}
