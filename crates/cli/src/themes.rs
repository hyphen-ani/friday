use ratatui::style::Color;

pub struct Theme {
    pub name:        &'static str,
    pub description: &'static str,
    pub accent:      Color,
    pub muted:       Color,
    pub text:        Color,
    pub dim:         Color,
    pub bg:          Color,
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "Ember",
        description: "warm amber on deep black",
        accent: Color::Rgb(230, 170, 80),
        muted:  Color::Rgb(110, 110, 110),
        text:   Color::White,
        dim:    Color::Rgb(80, 80, 80),
        bg:     Color::Rgb(16, 16, 16),
    },
    Theme {
        name: "Aurora",
        description: "glacial teal over obsidian",
        accent: Color::Rgb(70, 210, 155),
        muted:  Color::Rgb(70, 105, 90),
        text:   Color::Rgb(220, 245, 235),
        dim:    Color::Rgb(55, 80, 68),
        bg:     Color::Rgb(10, 18, 14),
    },
    Theme {
        name: "Crimson",
        description: "smoldering red on carbon black",
        accent: Color::Rgb(218, 65, 92),
        muted:  Color::Rgb(105, 55, 65),
        text:   Color::Rgb(245, 225, 228),
        dim:    Color::Rgb(78, 48, 54),
        bg:     Color::Rgb(18, 10, 12),
    },
    Theme {
        name: "Sakura",
        description: "cherry blossom pink on dark plum",
        accent: Color::Rgb(228, 102, 158),
        muted:  Color::Rgb(108, 60, 86),
        text:   Color::Rgb(248, 228, 238),
        dim:    Color::Rgb(80, 48, 65),
        bg:     Color::Rgb(18, 10, 15),
    },
    Theme {
        name: "Neptune",
        description: "deep sea blue on midnight black",
        accent: Color::Rgb(72, 142, 232),
        muted:  Color::Rgb(55, 76, 112),
        text:   Color::Rgb(220, 232, 250),
        dim:    Color::Rgb(45, 60, 92),
        bg:     Color::Rgb(10, 12, 22),
    },
    Theme {
        name: "Dusk",
        description: "twilight violet on deep purple-black",
        accent: Color::Rgb(162, 102, 242),
        muted:  Color::Rgb(85, 60, 118),
        text:   Color::Rgb(236, 226, 250),
        dim:    Color::Rgb(68, 48, 95),
        bg:     Color::Rgb(14, 10, 22),
    },
    Theme {
        name: "Venom",
        description: "neon green on absolute black",
        accent: Color::Rgb(50, 222, 100),
        muted:  Color::Rgb(38, 95, 58),
        text:   Color::Rgb(200, 240, 215),
        dim:    Color::Rgb(30, 72, 44),
        bg:     Color::Rgb(8, 14, 10),
    },
    Theme {
        name: "Obsidian",
        description: "minimal silver on obsidian",
        accent: Color::Rgb(190, 200, 215),
        muted:  Color::Rgb(80, 86, 96),
        text:   Color::Rgb(230, 236, 244),
        dim:    Color::Rgb(60, 65, 75),
        bg:     Color::Rgb(12, 12, 15),
    },
];

/// Returns the active theme — uses the picker-selected theme while the
/// picker is open (live preview), confirmed theme otherwise.
pub fn current(app: &crate::app::App) -> &'static Theme {
    let idx = if app.theme_picker_open {
        app.theme_picker_selected
    } else {
        app.theme_idx
    };
    &THEMES[idx.min(THEMES.len() - 1)]
}

pub fn by_name(name: &str) -> usize {
    THEMES.iter().position(|t| t.name == name).unwrap_or(0)
}
