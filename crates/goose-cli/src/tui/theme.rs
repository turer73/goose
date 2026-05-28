use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
pub struct Theme;

#[allow(dead_code)]
impl Theme {
    // Primary palette - warmer, more saturated
    pub const CRANBERRY: Color = Color::Rgb(219, 68, 96);
    pub const TEAL: Color = Color::Rgb(72, 185, 175);
    pub const GOLD: Color = Color::Rgb(228, 168, 72);

    // Text hierarchy - better contrast
    pub const TEXT_PRIMARY: Color = Color::Rgb(240, 237, 232);
    pub const TEXT_SECONDARY: Color = Color::Rgb(158, 175, 196);
    pub const TEXT_DIM: Color = Color::Rgb(82, 99, 120);

    // Surfaces - slightly warmer darks
    pub const BG_PRIMARY: Color = Color::Rgb(18, 22, 30);
    pub const BG_SURFACE: Color = Color::Rgb(24, 30, 42);
    pub const BG_ELEVATED: Color = Color::Rgb(32, 40, 56);
    pub const BORDER: Color = Color::Rgb(42, 56, 76);
    pub const BORDER_ACTIVE: Color = Color::Rgb(82, 108, 146);

    // Semantic
    pub const SUCCESS: Color = Color::Rgb(72, 185, 175);
    pub const ERROR: Color = Color::Rgb(219, 68, 96);
    pub const WARNING: Color = Color::Rgb(228, 168, 72);
    pub const INFO: Color = Color::Rgb(110, 160, 240);

    // Styles
    pub fn title() -> Style {
        Style::default()
            .fg(Self::TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtitle() -> Style {
        Style::default().fg(Self::TEXT_SECONDARY)
    }

    pub fn dim() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    pub fn accent() -> Style {
        Style::default()
            .fg(Self::CRANBERRY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    pub fn error() -> Style {
        Style::default().fg(Self::ERROR)
    }

    pub fn selected() -> Style {
        Style::default()
            .fg(Self::TEXT_PRIMARY)
            .bg(Self::BG_ELEVATED)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::BORDER)
    }

    pub fn border_active() -> Style {
        Style::default().fg(Self::BORDER_ACTIVE)
    }

    pub fn input_prompt() -> Style {
        Style::default()
            .fg(Self::TEAL)
            .add_modifier(Modifier::BOLD)
    }
}
