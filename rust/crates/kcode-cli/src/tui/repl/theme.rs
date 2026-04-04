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

    pub fn palette(&self) -> ThemePalette {
        match self {
            ThemePreset::Default => ThemePalette {
                accent: Color::Green,
                accent_soft: Color::Rgb(40, 120, 70),
                accent_dim: Color::Rgb(30, 80, 50),
                panel_bg: Color::Rgb(18, 28, 20),
                input_bg: Color::Rgb(12, 18, 12),
                text: Color::White,
                text_muted: Color::Gray,
                error: Color::Red,
                warning: Color::Yellow,
                success: Color::Green,
                info: Color::Cyan,
                border: Color::Rgb(60, 80, 60),
                selection_bg: Color::Rgb(30, 40, 30),
                dialog_bg: Color::Rgb(12, 16, 12),
                user_msg_bg: Color::Rgb(25, 35, 25),
                assistant_msg_bg: Color::Rgb(18, 22, 18),
            },
            ThemePreset::Amber => ThemePalette {
                accent: Color::Yellow,
                accent_soft: Color::Rgb(210, 160, 30),
                accent_dim: Color::Rgb(150, 110, 20),
                panel_bg: Color::Rgb(42, 30, 8),
                input_bg: Color::Rgb(30, 22, 5),
                text: Color::White,
                text_muted: Color::Gray,
                error: Color::Red,
                warning: Color::Rgb(255, 165, 0),
                success: Color::Yellow,
                info: Color::Rgb(100, 200, 255),
                border: Color::Rgb(180, 140, 50),
                selection_bg: Color::Rgb(50, 35, 10),
                dialog_bg: Color::Rgb(20, 15, 5),
                user_msg_bg: Color::Rgb(45, 35, 12),
                assistant_msg_bg: Color::Rgb(35, 28, 10),
            },
            ThemePreset::Ocean => ThemePalette {
                accent: Color::Cyan,
                accent_soft: Color::Rgb(40, 150, 170),
                accent_dim: Color::Rgb(25, 100, 120),
                panel_bg: Color::Rgb(8, 32, 42),
                input_bg: Color::Rgb(5, 22, 30),
                text: Color::White,
                text_muted: Color::Gray,
                error: Color::Red,
                warning: Color::Yellow,
                success: Color::Cyan,
                info: Color::Rgb(100, 200, 255),
                border: Color::Rgb(30, 80, 100),
                selection_bg: Color::Rgb(12, 40, 50),
                dialog_bg: Color::Rgb(5, 15, 22),
                user_msg_bg: Color::Rgb(12, 35, 45),
                assistant_msg_bg: Color::Rgb(8, 28, 35),
            },
            ThemePreset::DarkHighContrast => ThemePalette {
                accent: Color::Rgb(0, 255, 0),
                accent_soft: Color::Rgb(0, 200, 0),
                accent_dim: Color::Rgb(0, 150, 0),
                panel_bg: Color::Black,
                input_bg: Color::Rgb(5, 5, 5),
                text: Color::Rgb(255, 255, 255),
                text_muted: Color::Rgb(200, 200, 200),
                error: Color::Rgb(255, 80, 80),
                warning: Color::Rgb(255, 255, 0),
                success: Color::Rgb(0, 255, 0),
                info: Color::Rgb(100, 200, 255),
                border: Color::Rgb(150, 150, 150),
                selection_bg: Color::Rgb(50, 50, 50),
                dialog_bg: Color::Black,
                user_msg_bg: Color::Rgb(30, 30, 30),
                assistant_msg_bg: Color::Rgb(20, 20, 20),
            },
            ThemePreset::CatppuccinMocha => ThemePalette {
                accent: Color::Rgb(166, 227, 161),
                accent_soft: Color::Rgb(137, 180, 137),
                accent_dim: Color::Rgb(100, 140, 100),
                panel_bg: Color::Rgb(30, 30, 46),
                input_bg: Color::Rgb(24, 24, 37),
                text: Color::Rgb(205, 214, 244),
                text_muted: Color::Rgb(127, 132, 156),
                error: Color::Rgb(243, 139, 168),
                warning: Color::Rgb(249, 226, 175),
                success: Color::Rgb(166, 227, 161),
                info: Color::Rgb(137, 180, 250),
                border: Color::Rgb(88, 91, 112),
                selection_bg: Color::Rgb(58, 58, 82),
                dialog_bg: Color::Rgb(17, 17, 27),
                user_msg_bg: Color::Rgb(35, 35, 55),
                assistant_msg_bg: Color::Rgb(28, 28, 42),
            },
            ThemePreset::Light => ThemePalette {
                accent: Color::Rgb(0, 100, 0),
                accent_soft: Color::Rgb(0, 130, 0),
                accent_dim: Color::Rgb(0, 160, 0),
                panel_bg: Color::Rgb(245, 245, 245),
                input_bg: Color::Rgb(250, 250, 250),
                text: Color::Rgb(20, 20, 20),
                text_muted: Color::Rgb(100, 100, 100),
                error: Color::Rgb(180, 0, 0),
                warning: Color::Rgb(180, 140, 0),
                success: Color::Rgb(0, 120, 0),
                info: Color::Rgb(0, 100, 180),
                border: Color::Rgb(200, 200, 200),
                selection_bg: Color::Rgb(220, 230, 220),
                dialog_bg: Color::Rgb(240, 240, 240),
                user_msg_bg: Color::Rgb(235, 240, 235),
                assistant_msg_bg: Color::Rgb(240, 240, 240),
            },
        }
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
    pub accent: Color,
    pub accent_soft: Color,
    pub accent_dim: Color,
    pub panel_bg: Color,
    pub input_bg: Color,
    pub text: Color,
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
