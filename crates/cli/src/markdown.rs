use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};

const ACCENT: Color = Color::Rgb(163, 230, 53);
const TEXT:   Color = Color::White;
const DIM:    Color = Color::Rgb(100, 100, 100);
const CODE_BG: Color = Color::Rgb(28, 28, 28);

pub fn to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    let mut in_code_block = false;
    let mut in_strong = false;
    let mut in_em = false;
    let mut heading_level: Option<HeadingLevel> = None;

    let parser = Parser::new_ext(text, Options::empty());

    for event in parser {
        match event {
            // ── Code blocks ──────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut lines, &mut current_spans);
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::from(""));
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
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(TEXT).bg(CODE_BG),
                        )));
                    }
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
    lines
}

fn flush(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}
