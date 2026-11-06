use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const ACCENT:  Color = Color::Rgb(163, 230, 53);
const TEXT:    Color = Color::White;
const DIM:     Color = Color::Rgb(100, 100, 100);
const CODE_BG: Color = Color::Rgb(28, 28, 28);

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let mut ts = ThemeSet::load_defaults();
        ts.themes
            .remove("base16-ocean.dark")
            .expect("base16-ocean.dark is a built-in syntect theme")
    })
}

pub fn to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    let mut in_code_block = false;
    let mut code_lang     = String::new();
    let mut pending_code  = String::new();

    let mut in_strong = false;
    let mut in_em     = false;
    let mut heading_level: Option<HeadingLevel> = None;

    let parser = Parser::new_ext(text, Options::empty());

    for event in parser {
        match event {
            // ── Code blocks ──────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut lines, &mut current_spans);
                in_code_block = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented     => String::new(),
                };
                pending_code.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                // Trim the trailing newline pulldown-cmark always appends
                let code = pending_code.trim_end_matches('\n');
                lines.extend(highlight_block(code, &code_lang));
                lines.push(Line::from(""));
                in_code_block = false;
                code_lang.clear();
                pending_code.clear();
            }

            // ── Inline formatting ─────────────────────────────────────────────
            Event::Start(Tag::Strong)    => in_strong = true,
            Event::End(TagEnd::Strong)   => in_strong = false,
            Event::Start(Tag::Emphasis)  => in_em = true,
            Event::End(TagEnd::Emphasis) => in_em = false,

            // ── Headings ──────────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut lines, &mut current_spans);
                heading_level = Some(level);
                let prefix = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    HeadingLevel::H3 => "### ",
                    _                => "",
                };
                if !prefix.is_empty() {
                    current_spans.push(Span::styled(
                        prefix,
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ));
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(&mut lines, &mut current_spans);
                lines.push(Line::from(""));
                heading_level = None;
            }

            // ── Paragraphs ────────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush(&mut lines, &mut current_spans);
                lines.push(Line::from(""));
            }

            // ── Lists ─────────────────────────────────────────────────────────
            Event::Start(Tag::Item) => {
                flush(&mut lines, &mut current_spans);
                current_spans.push(Span::styled("  • ", Style::default().fg(ACCENT)));
            }
            Event::End(TagEnd::Item) => {
                flush(&mut lines, &mut current_spans);
            }

            // ── Inline code ───────────────────────────────────────────────────
            Event::Code(text) => {
                current_spans.push(Span::styled(
                    format!("`{}`", text),
                    Style::default().fg(ACCENT),
                ));
            }

            // ── Text ──────────────────────────────────────────────────────────
            Event::Text(text) => {
                if in_code_block {
                    pending_code.push_str(&text);
                } else {
                    let mut style = Style::default();
                    style = style.fg(if heading_level.is_some() { ACCENT } else { TEXT });
                    if in_strong || heading_level.is_some() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if in_em {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }

            // ── Breaks ────────────────────────────────────────────────────────
            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                flush(&mut lines, &mut current_spans);
            }

            // ── Rule ──────────────────────────────────────────────────────────
            Event::Rule => {
                flush(&mut lines, &mut current_spans);
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(DIM),
                )));
                lines.push(Line::from(""));
            }

            _ => {}
        }
    }

    flush(&mut lines, &mut current_spans);

    // If the stream ended mid-code-block (no closing fence yet), emit what
    // we have as plain text so the content is visible while streaming.
    if in_code_block && !pending_code.is_empty() {
        for raw in pending_code.trim_end_matches('\n').lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", raw),
                Style::default().fg(DIM).bg(CODE_BG),
            )));
        }
    }

    lines
}

/// Syntax-highlight a code block, falling back to plain dim text for unknown languages.
fn highlight_block(code: &str, lang: &str) -> Vec<Line<'static>> {
    let ss = syntax_set();
    let syntax = if lang.is_empty() {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let use_highlighting = syntax.name != "Plain Text";
    let mut h = HighlightLines::new(syntax, theme());
    let mut result = Vec::new();

    for raw_line in LinesWithEndings::from(code) {
        if use_highlighting {
            let ranges = h.highlight_line(raw_line, ss).unwrap_or_default();
            let mut spans: Vec<Span<'static>> =
                vec![Span::styled("  ", Style::default().bg(CODE_BG))];

            for (hl_style, fragment) in &ranges {
                let owned = fragment.trim_end_matches('\n').to_string();
                if owned.is_empty() {
                    continue;
                }
                let fg = hl_style.foreground;
                spans.push(Span::styled(
                    owned,
                    Style::default()
                        .fg(Color::Rgb(fg.r, fg.g, fg.b))
                        .bg(CODE_BG),
                ));
            }

            result.push(Line::from(spans));
        } else {
            let trimmed = raw_line.trim_end_matches('\n').to_string();
            result.push(Line::from(Span::styled(
                format!("  {}", trimmed),
                Style::default().fg(DIM).bg(CODE_BG),
            )));
        }
    }

    result
}

fn flush(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}
