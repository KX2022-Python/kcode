use ratatui::style::Color;

/// 主题预设 — 对齐 CC-Haha 主题系统 + 终端自适应
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    /// 默认暗色主题（绿色系）
    Default,
    /// 琥珀色主题
    Amber,
    /// 海洋色主题
    Ocean,
    /// 高对比度暗色
    DarkHighContrast,
    /// Catppuccin Mocha
    CatppuccinMocha,
    /// 浅色主题
    Light,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 6] = [
        ThemePreset::Default,
        ThemePreset::Amber,
        ThemePreset::Ocean,
        ThemePreset::CatppuccinMocha,
        ThemePreset::DarkHighContrast,
        ThemePreset::Light,
    ];

    pub fn name(&self) -> &str {
        match self {
            ThemePreset::Default => "default",
            ThemePreset::Amber => "amber",
            ThemePreset::Ocean => "ocean",
            ThemePreset::DarkHighContrast => "dark-hc",
            ThemePreset::CatppuccinMocha => "catppuccin",
            ThemePreset::Light => "light",
        }
    }

    pub fn label(&self) -> &str {
        self.name()
    }

    pub fn display_name(&self) -> &str {
        match self {
            ThemePreset::Default => "Dark mode",
            ThemePreset::Amber => "Amber mode",
            ThemePreset::Ocean => "Ocean mode",
            ThemePreset::DarkHighContrast => "Dark mode (high contrast)",
            ThemePreset::CatppuccinMocha => "Catppuccin Mocha",
            ThemePreset::Light => "Light mode",
        }
    }

    pub fn helper_text(&self) -> &str {
        "Codex CLI 风格：跟随终端默认底色，cyan 表示输入/状态/代码，magenta 表示品牌与命令。"
    }

    pub fn parse(value: &str) -> ThemePreset {
        match value.trim() {
            "amber" => ThemePreset::Amber,
            "ocean" => ThemePreset::Ocean,
            "dark-hc" => ThemePreset::DarkHighContrast,
            "catppuccin" => ThemePreset::CatppuccinMocha,
            "light" => ThemePreset::Light,
            _ => ThemePreset::Default,
        }
    }

    pub fn palette(&self) -> ThemePalette {
        codex_cli_palette(*self)
    }

    pub fn cycle(&self) -> ThemePreset {
        match self {
            ThemePreset::Default => ThemePreset::Amber,
            ThemePreset::Amber => ThemePreset::Ocean,
            ThemePreset::Ocean => ThemePreset::CatppuccinMocha,
            ThemePreset::CatppuccinMocha => ThemePreset::DarkHighContrast,
            ThemePreset::DarkHighContrast => ThemePreset::Light,
            ThemePreset::Light => ThemePreset::Default,
        }
    }
}

/// 主题调色板
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub brand: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub accent_dim: Color,
    pub panel_bg: Color,
    pub input_bg: Color,
    pub text: Color,
    pub inverse_text: Color,
    pub text_muted: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    pub border: Color,
    pub selection_bg: Color,
    pub dialog_bg: Color,
    pub user_msg_bg: Color,
    pub assistant_msg_bg: Color,
    pub prefers_light_code: bool,
}

fn codex_cli_palette(theme: ThemePreset) -> ThemePalette {
    ThemePalette {
        brand: Color::Magenta,
        accent: Color::Cyan,
        accent_soft: Color::Cyan,
        accent_dim: Color::Cyan,
        panel_bg: Color::Reset,
        input_bg: Color::Reset,
        text: Color::Reset,
        inverse_text: Color::Black,
        text_muted: Color::DarkGray,
        error: Color::Red,
        warning: Color::Cyan,
        success: Color::Green,
        info: Color::Cyan,
        border: Color::Cyan,
        selection_bg: Color::Reset,
        dialog_bg: Color::Reset,
        user_msg_bg: Color::Reset,
        assistant_msg_bg: Color::Reset,
        prefers_light_code: matches!(theme, ThemePreset::Light),
    }
}

/// 终端类型检测 — 对齐 CC-Haha 终端自适应
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalType {
    Xterm,
    ITerm2,
    WezTerm,
    GnomeTerminal,
    WindowsTerminal,
    Unknown,
}

impl TerminalType {
    /// 检测终端类型
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        if term_program.contains("iTerm") {
            return TerminalType::ITerm2;
        }
        if term_program.contains("WezTerm") {
            return TerminalType::WezTerm;
        }
        if term_program.contains("WindowsTerminal") {
            return TerminalType::WindowsTerminal;
        }
        if term.contains("xterm") || term.contains("alacritty") {
            return TerminalType::Xterm;
        }
        if term.contains("gnome") {
            return TerminalType::GnomeTerminal;
        }
        TerminalType::Unknown
    }

    /// 是否支持真彩色
    pub fn supports_true_color(&self) -> bool {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        colorterm.contains("truecolor") || colorterm.contains("24bit")
    }

    /// 推荐的默认主题
    pub fn recommended_theme(&self) -> ThemePreset {
        if !self.supports_true_color() {
            // 256 色终端使用简化主题
            return ThemePreset::Default;
        }
        ThemePreset::Default
    }
}
