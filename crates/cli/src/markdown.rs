use std::sync::OnceLock;

use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const ACCENT:  Color = Color::Rgb(230, 170, 80);
const TEXT:    Color = Color::White;
const DIM:     Color = Color::Rgb(100, 100, 100);
const CODE_BG: Color = Color::Rgb(28, 28, 28);
const MATH_BG: Color = Color::Rgb(20, 22, 34);

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME:      OnceLock<Theme>     = OnceLock::new();

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

    // ── Code block state ─────────────────────────────────
    let mut in_code_block = false;
    let mut code_lang     = String::new();
    let mut pending_code  = String::new();

    // ── Inline formatting state ───────────────────────────
    let mut in_strong = false;
    let mut in_em     = false;
    let mut heading_level: Option<HeadingLevel> = None;

    // ── Table state ───────────────────────────────────────
    let mut table_header:     Vec<String>       = Vec::new();
    let mut table_body:       Vec<Vec<String>>  = Vec::new();
    let mut current_row:      Vec<String>       = Vec::new();
    let mut current_cell:     String            = String::new();
    let mut in_table_head = false;
    let mut in_table_cell = false;
    let mut table_alignments: Vec<Alignment>   = Vec::new();

    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_MATH
        | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(text, opts);

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
                let code = pending_code.trim_end_matches('\n');
                lines.extend(highlight_block(code, &code_lang));
                lines.push(Line::from(""));
                in_code_block = false;
                code_lang.clear();
                pending_code.clear();
            }

            // ── Tables ───────────────────────────────────────────────────────
            Event::Start(Tag::Table(aligns)) => {
                flush(&mut lines, &mut current_spans);
                table_header.clear();
                table_body.clear();
                current_row.clear();
                table_alignments = aligns.to_vec();
            }
            Event::End(TagEnd::Table) => {
                lines.extend(render_table(&table_header, &table_body, &table_alignments));
                lines.push(Line::from(""));
                in_table_head = false;
                in_table_cell = false;
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if in_table_head {
                    table_header = current_row.drain(..).collect();
                } else {
                    table_body.push(current_row.drain(..).collect());
                }
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
                in_table_cell = true;
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.trim().to_string());
                current_cell.clear();
                in_table_cell = false;
            }

            // ── Math ─────────────────────────────────────────────────────────
            Event::InlineMath(math) => {
                let rendered = latex_to_unicode(&math);
                current_spans.push(Span::styled(
                    format!(" {} ", rendered),
                    Style::default().fg(ACCENT).bg(MATH_BG),
                ));
            }
            Event::DisplayMath(math) => {
                flush(&mut lines, &mut current_spans);
                let rendered = latex_to_unicode(&math);
                lines.push(Line::from(Span::styled(
                    "  ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌".to_string(),
                    Style::default().fg(DIM),
                )));
                for ln in rendered.trim().lines() {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", ln),
                        Style::default().fg(ACCENT).bg(MATH_BG).add_modifier(Modifier::BOLD),
                    )));
                }
                lines.push(Line::from(Span::styled(
                    "  ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌".to_string(),
                    Style::default().fg(DIM),
                )));
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
                    pending_code.push_str(&text);
                } else if in_table_cell {
                    current_cell.push_str(&text);
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
                if !in_table_cell {
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                if !in_table_cell {
                    flush(&mut lines, &mut current_spans);
                }
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

    // Flush incomplete code block (streaming: no closing fence yet)
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

// ── Table rendering ───────────────────────────────────────────────────────────

fn render_table(
    header: &[String],
    body: &[Vec<String>],
    alignments: &[Alignment],
) -> Vec<Line<'static>> {
    let col_count = header.len()
        .max(body.iter().map(|r| r.len()).max().unwrap_or(0));
    if col_count == 0 {
        return vec![];
    }

    // Compute per-column widths (minimum 1)
    let mut widths: Vec<usize> = vec![1; col_count];
    for (i, cell) in header.iter().enumerate() {
        widths[i] = widths[i].max(cell.chars().count());
    }
    for row in body {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
    }

    let mut out: Vec<Line<'static>> = Vec::new();
    out.push(h_border(&widths, '┌', '─', '┬', '┐'));
    out.push(data_row(header, &widths, alignments, true));
    out.push(h_border(&widths, '├', '─', '┼', '┤'));
    for row in body {
        out.push(data_row(row, &widths, alignments, false));
    }
    out.push(h_border(&widths, '└', '─', '┴', '┘'));
    out
}

fn h_border(widths: &[usize], l: char, fill: char, mid: char, r: char) -> Line<'static> {
    let mut s = String::new();
    s.push(l);
    for (i, &w) in widths.iter().enumerate() {
        for _ in 0..w + 2 { s.push(fill); }
        s.push(if i + 1 < widths.len() { mid } else { r });
    }
    Line::from(Span::styled(s, Style::default().fg(DIM)))
}

fn data_row(
    cells: &[String],
    widths: &[usize],
    alignments: &[Alignment],
    is_header: bool,
) -> Line<'static> {
    let border = Span::styled("│".to_string(), Style::default().fg(DIM));
    let mut spans: Vec<Span<'static>> = vec![border.clone()];

    for (i, &w) in widths.iter().enumerate() {
        let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
        let align = alignments.get(i).copied().unwrap_or(Alignment::None);
        let padded = match align {
            Alignment::Right  => format!(" {:>w$} ", cell, w = w),
            Alignment::Center => format!(" {:^w$} ", cell, w = w),
            _                 => format!(" {:<w$} ", cell, w = w),
        };
        let style = if is_header {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT)
        };
        spans.push(Span::styled(padded, style));
        spans.push(border.clone());
    }

    Line::from(spans)
}

// ── LaTeX → Unicode ───────────────────────────────────────────────────────────

fn latex_to_unicode(s: &str) -> String {
    const SYMS: &[(&str, &str)] = &[
        // Greek lower
        ("\\alpha","α"),("\\beta","β"),("\\gamma","γ"),("\\delta","δ"),
        ("\\epsilon","ε"),("\\varepsilon","ε"),("\\zeta","ζ"),("\\eta","η"),
        ("\\theta","θ"),("\\vartheta","ϑ"),("\\iota","ι"),("\\kappa","κ"),
        ("\\lambda","λ"),("\\mu","μ"),("\\nu","ν"),("\\xi","ξ"),
        ("\\pi","π"),("\\varpi","ϖ"),("\\rho","ρ"),("\\varrho","ϱ"),
        ("\\sigma","σ"),("\\varsigma","ς"),("\\tau","τ"),("\\upsilon","υ"),
        ("\\phi","φ"),("\\varphi","ϕ"),("\\chi","χ"),("\\psi","ψ"),("\\omega","ω"),
        // Greek upper
        ("\\Gamma","Γ"),("\\Delta","Δ"),("\\Theta","Θ"),("\\Lambda","Λ"),
        ("\\Xi","Ξ"),("\\Pi","Π"),("\\Sigma","Σ"),("\\Upsilon","Υ"),
        ("\\Phi","Φ"),("\\Psi","Ψ"),("\\Omega","Ω"),
        // Operators / relations
        ("\\sum","∑"),("\\prod","∏"),("\\int","∫"),("\\iint","∬"),("\\iiint","∭"),
        ("\\oint","∮"),("\\infty","∞"),("\\partial","∂"),("\\nabla","∇"),
        ("\\sqrt","√"),("\\pm","±"),("\\mp","∓"),("\\times","×"),("\\div","÷"),
        ("\\cdot","·"),("\\leq","≤"),("\\le","≤"),("\\geq","≥"),("\\ge","≥"),
        ("\\neq","≠"),("\\ne","≠"),("\\approx","≈"),("\\equiv","≡"),
        ("\\sim","∼"),("\\simeq","≃"),("\\cong","≅"),("\\propto","∝"),
        ("\\ll","≪"),("\\gg","≫"),("\\prec","≺"),("\\succ","≻"),
        ("\\in","∈"),("\\notin","∉"),("\\ni","∋"),
        ("\\subset","⊂"),("\\supset","⊃"),("\\subseteq","⊆"),("\\supseteq","⊇"),
        ("\\cup","∪"),("\\cap","∩"),("\\setminus","∖"),
        ("\\forall","∀"),("\\exists","∃"),("\\nexists","∄"),("\\emptyset","∅"),
        ("\\to","→"),("\\gets","←"),("\\leftarrow","←"),("\\rightarrow","→"),
        ("\\Leftarrow","⇐"),("\\Rightarrow","⇒"),("\\Leftrightarrow","⟺"),
        ("\\leftrightarrow","↔"),("\\uparrow","↑"),("\\downarrow","↓"),
        ("\\nearrow","↗"),("\\searrow","↘"),("\\nwarrow","↖"),("\\swarrow","↙"),
        ("\\ldots","…"),("\\cdots","⋯"),("\\vdots","⋮"),("\\ddots","⋱"),
        ("\\ell","ℓ"),("\\hbar","ℏ"),("\\Re","ℜ"),("\\Im","ℑ"),
        ("\\angle","∠"),("\\perp","⊥"),("\\parallel","∥"),("\\mid","∣"),
        ("\\oplus","⊕"),("\\ominus","⊖"),("\\otimes","⊗"),("\\oslash","⊘"),
        ("\\circ","∘"),("\\bullet","•"),("\\star","⋆"),("\\dagger","†"),
        ("\\land","∧"),("\\lor","∨"),("\\lnot","¬"),("\\neg","¬"),
        ("\\because","∵"),("\\therefore","∴"),
        ("\\langle","⟨"),("\\rangle","⟩"),
        ("\\lceil","⌈"),("\\rceil","⌉"),("\\lfloor","⌊"),("\\rfloor","⌋"),
        ("\\aleph","ℵ"),
        // Delimiters / spacing
        ("\\left(","("),("\\right)",")"),
        ("\\left[","["),("\\right]","]"),
        ("\\left\\{","{"),("\\right\\}","}"),
        ("\\{","{"),("\\}","}"),("\\%","%"),("\\_","_"),
        ("\\,",""),("\\;",""),("\\!"," "),("\\quad"," "),("\\qquad","  "),
    ];

    let mut r = s.to_string();

    // \frac{num}{den} → num⁄den
    r = replace_frac(&r);

    // \text{...}, \mathrm{...} etc. → content only
    for cmd in &[
        "\\text","\\mathrm","\\mathbf","\\mathit",
        "\\mathsf","\\mathbb","\\mathcal","\\operatorname",
    ] {
        r = strip_command_braces(&r, cmd);
    }

    // Symbol substitution
    for (from, to) in SYMS {
        r = r.replace(from, to);
    }

    // Super- and subscripts
    r = replace_scripts(&r, '^', super_char);
    r = replace_scripts(&r, '_', sub_char);

    // Strip leftover braces and unknown \commands
    r = r.replace('{', "").replace('}', "");
    r = strip_unknown_commands(&r);

    r.trim().to_string()
}

/// Replace `\frac{num}{den}` with `num⁄den`.
fn replace_frac(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < s.len() {
        if s[pos..].starts_with("\\frac") {
            let after = pos + 5;
            if s[after..].starts_with('{') {
                if let Some((num, e1)) = extract_balanced(&s[after..]) {
                    let next = after + e1;
                    if s[next..].starts_with('{') {
                        if let Some((den, e2)) = extract_balanced(&s[next..]) {
                            result.push_str(&num);
                            result.push('⁄');
                            result.push_str(&den);
                            pos = next + e2;
                            continue;
                        }
                    }
                }
            }
        }
        let c = s[pos..].chars().next().unwrap();
        result.push(c);
        pos += c.len_utf8();
    }
    result
}

/// Replace `\cmd{content}` with just `content`.
fn strip_command_braces(s: &str, cmd: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < s.len() {
        if s[pos..].starts_with(cmd) {
            let after = pos + cmd.len();
            if s[after..].starts_with('{') {
                if let Some((content, consumed)) = extract_balanced(&s[after..]) {
                    result.push_str(&content);
                    pos = after + consumed;
                    continue;
                }
            }
        }
        let c = s[pos..].chars().next().unwrap();
        result.push(c);
        pos += c.len_utf8();
    }
    result
}

/// Extract content of the balanced `{...}` group at the start of `s`.
/// Returns `(content, bytes_consumed)` or `None`.
fn extract_balanced(s: &str) -> Option<(String, usize)> {
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    let mut content = String::new();
    let mut consumed = 0usize;
    for c in s.chars() {
        consumed += c.len_utf8();
        match c {
            '{' => { depth += 1; if depth > 1 { content.push(c); } }
            '}' => {
                depth -= 1;
                if depth == 0 { return Some((content, consumed)); }
                content.push(c);
            }
            _ => content.push(c),
        }
    }
    None
}

/// Convert `^x`, `^{...}`, `_x`, `_{...}` to Unicode super/subscript characters.
fn replace_scripts(s: &str, marker: char, convert: fn(char) -> char) -> String {
    let mut result = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < s.len() {
        let c = s[pos..].chars().next().unwrap();
        if c == marker {
            pos += c.len_utf8();
            if s[pos..].starts_with('{') {
                if let Some((content, consumed)) = extract_balanced(&s[pos..]) {
                    for ch in content.chars() {
                        result.push(convert(ch));
                    }
                    pos += consumed;
                    continue;
                }
            } else if let Some(nc) = s[pos..].chars().next() {
                if nc.is_alphanumeric() || "+-=()".contains(nc) {
                    result.push(convert(nc));
                    pos += nc.len_utf8();
                    continue;
                }
            }
            result.push(marker);
        } else {
            result.push(c);
            pos += c.len_utf8();
        }
    }
    result
}

/// Remove any remaining `\word` commands not matched by the symbol table.
fn strip_unknown_commands(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < s.len() {
        let c = s[pos..].chars().next().unwrap();
        if c == '\\' {
            let start = pos + 1;
            let end = s[start..]
                .find(|ch: char| !ch.is_alphabetic())
                .map(|i| start + i)
                .unwrap_or(s.len());
            if end > start {
                pos = end;
                if pos < s.len() && s[pos..].starts_with(' ') {
                    pos += 1;
                }
                continue;
            }
        }
        result.push(c);
        pos += c.len_utf8();
    }
    result
}

fn super_char(c: char) -> char {
    match c {
        '0' => '⁰', '1' => '¹', '2' => '²', '3' => '³', '4' => '⁴',
        '5' => '⁵', '6' => '⁶', '7' => '⁷', '8' => '⁸', '9' => '⁹',
        'a' => 'ᵃ', 'b' => 'ᵇ', 'c' => 'ᶜ', 'd' => 'ᵈ', 'e' => 'ᵉ',
        'f' => 'ᶠ', 'g' => 'ᵍ', 'h' => 'ʰ', 'i' => 'ⁱ', 'j' => 'ʲ',
        'k' => 'ᵏ', 'l' => 'ˡ', 'm' => 'ᵐ', 'n' => 'ⁿ', 'o' => 'ᵒ',
        'p' => 'ᵖ', 'r' => 'ʳ', 's' => 'ˢ', 't' => 'ᵗ', 'u' => 'ᵘ',
        'v' => 'ᵛ', 'w' => 'ʷ', 'x' => 'ˣ', 'y' => 'ʸ', 'z' => 'ᶻ',
        'A' => 'ᴬ', 'B' => 'ᴮ', 'D' => 'ᴰ', 'E' => 'ᴱ', 'G' => 'ᴳ',
        'H' => 'ᴴ', 'I' => 'ᴵ', 'J' => 'ᴶ', 'K' => 'ᴷ', 'L' => 'ᴸ',
        'M' => 'ᴹ', 'N' => 'ᴺ', 'O' => 'ᴼ', 'P' => 'ᴾ', 'R' => 'ᴿ',
        'T' => 'ᵀ', 'U' => 'ᵁ', 'V' => 'ⱽ', 'W' => 'ᵂ',
        '+' => '⁺', '-' => '⁻', '=' => '⁼', '(' => '⁽', ')' => '⁾',
        _ => c,
    }
}

fn sub_char(c: char) -> char {
    match c {
        '0' => '₀', '1' => '₁', '2' => '₂', '3' => '₃', '4' => '₄',
        '5' => '₅', '6' => '₆', '7' => '₇', '8' => '₈', '9' => '₉',
        'a' => 'ₐ', 'e' => 'ₑ', 'o' => 'ₒ', 'x' => 'ₓ', 'h' => 'ₕ',
        'i' => 'ᵢ', 'j' => 'ⱼ', 'k' => 'ₖ', 'l' => 'ₗ', 'm' => 'ₘ',
        'n' => 'ₙ', 'p' => 'ₚ', 'r' => 'ᵣ', 's' => 'ₛ', 't' => 'ₜ',
        'u' => 'ᵤ', 'v' => 'ᵥ',
        '+' => '₊', '-' => '₋', '=' => '₌', '(' => '₍', ')' => '₎',
        _ => c,
    }
}

// ── Syntax highlighting ───────────────────────────────────────────────────────

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
                if owned.is_empty() { continue; }
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
