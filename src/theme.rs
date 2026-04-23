use ratatui::style::Color;
use std::sync::atomic::{AtomicU8, Ordering};
use syntect::{highlighting::Theme, highlighting::ThemeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ThemePreset {
    Arctic = 0,
    Forest = 1,
    OceanDark = 2,
    SolarizedDark = 3,
}

impl Default for ThemePreset {
    fn default() -> Self {
        DEFAULT_PRESET
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AppTheme {
    pub(crate) syntax_theme_name: &'static str,
    pub(crate) ui: UiTheme,
    pub(crate) markdown: MarkdownTheme,
}

#[derive(Clone, Copy)]
pub(crate) struct UiTheme {
    pub(crate) toc_bg: Color,
    pub(crate) toc_border: Color,
    pub(crate) content_bg: Color,
    pub(crate) scrollbar_hover: Color,
    pub(crate) status_bg: Color,
    pub(crate) status_separator: Color,
    pub(crate) status_brand_fg: Color,
    pub(crate) status_brand_bg: Color,
    pub(crate) status_filename_fg: Color,
    pub(crate) status_filename_bg: Color,
    pub(crate) status_watch_fg: Color,
    pub(crate) status_watch_bg: Color,
    pub(crate) status_reloaded_fg: Color,
    pub(crate) status_reloaded_bg: Color,
    pub(crate) status_search_fg: Color,
    pub(crate) status_search_bg: Color,
    pub(crate) status_success_fg: Color,
    pub(crate) status_success_bg: Color,
    pub(crate) status_warning_fg: Color,
    pub(crate) status_error_fg: Color,
    pub(crate) status_error_bg: Color,
    pub(crate) status_shortcut_fg: Color,
    pub(crate) status_percent_fg: Color,
    pub(crate) toc_header_fg: Color,
    pub(crate) toc_active_bg: Color,
    pub(crate) toc_inactive_bg: Color,
    pub(crate) toc_accent: Color,
    pub(crate) toc_index_inactive: Color,
    pub(crate) toc_primary_active: Color,
    pub(crate) toc_primary_inactive: Color,
    pub(crate) toc_secondary_inactive: Color,
    pub(crate) toc_secondary_text_active: Color,
    pub(crate) toc_secondary_text_inactive: Color,
}

#[derive(Clone, Copy)]
pub(crate) struct MarkdownTheme {
    pub(crate) search_highlight_bg: Color,
    pub(crate) code_gutter: Color,
    pub(crate) blockquote_marker: Color,
    pub(crate) list_level_1: Color,
    pub(crate) list_level_2: Color,
    pub(crate) list_level_3: Color,
    pub(crate) ordered_list: Color,
    pub(crate) table_border: Color,
    pub(crate) table_separator: Color,
    pub(crate) table_header: Color,
    pub(crate) table_cell: Color,
    pub(crate) heading_1: Color,
    pub(crate) heading_2: Color,
    pub(crate) heading_3: Color,
    pub(crate) heading_other: Color,
    pub(crate) heading_underline: Color,
    pub(crate) code_frame: Color,
    pub(crate) code_label: Color,
    pub(crate) inline_code_fg: Color,
    pub(crate) inline_code_bg: Color,
    pub(crate) rule: Color,
    pub(crate) link_icon: Color,
    pub(crate) link_text: Color,
    pub(crate) blockquote_text: Color,
    pub(crate) text: Color,
    pub(crate) strong_text: Color,
    pub(crate) latex_inline_fg: Color,
    pub(crate) latex_inline_bg: Color,
    pub(crate) latex_block_fg: Color,
}

const BASE_LIGHT_UI: UiTheme = UiTheme {
    toc_bg: Color::Rgb(232, 239, 245),
    toc_border: Color::Rgb(170, 182, 194),
    content_bg: Color::Rgb(242, 247, 250),
    scrollbar_hover: Color::Rgb(76, 122, 168),
    status_bg: Color::Rgb(224, 233, 240),
    status_separator: Color::Rgb(108, 126, 144),
    status_brand_fg: Color::Rgb(245, 248, 250),
    status_brand_bg: Color::Rgb(76, 122, 168),
    status_filename_fg: Color::Rgb(58, 84, 110),
    status_filename_bg: Color::Rgb(209, 221, 232),
    status_watch_fg: Color::Rgb(48, 140, 98),
    status_watch_bg: Color::Rgb(212, 234, 222),
    status_reloaded_fg: Color::Rgb(245, 248, 250),
    status_reloaded_bg: Color::Rgb(58, 168, 116),
    status_search_fg: Color::Rgb(142, 114, 24),
    status_search_bg: Color::Rgb(240, 234, 200),
    status_success_fg: Color::Rgb(48, 140, 98),
    status_success_bg: Color::Rgb(212, 234, 222),
    status_warning_fg: Color::Rgb(180, 142, 28),
    status_error_fg: Color::Rgb(188, 74, 74),
    status_error_bg: Color::Rgb(240, 218, 218),
    status_shortcut_fg: Color::Rgb(98, 116, 134),
    status_percent_fg: Color::Rgb(76, 122, 168),
    toc_header_fg: Color::Rgb(92, 108, 126),
    toc_active_bg: Color::Rgb(214, 224, 233),
    toc_inactive_bg: Color::Rgb(232, 239, 245),
    toc_accent: Color::Rgb(76, 122, 168),
    toc_index_inactive: Color::Rgb(126, 138, 152),
    toc_primary_active: Color::Rgb(34, 42, 52),
    toc_primary_inactive: Color::Rgb(82, 96, 110),
    toc_secondary_inactive: Color::Rgb(126, 138, 152),
    toc_secondary_text_active: Color::Rgb(48, 58, 68),
    toc_secondary_text_inactive: Color::Rgb(98, 112, 126),
};

const BASE_DARK_UI: UiTheme = UiTheme {
    toc_bg: Color::Rgb(18, 18, 22),
    toc_border: Color::Rgb(52, 52, 58),
    content_bg: Color::Rgb(18, 20, 28),
    scrollbar_hover: Color::Rgb(105, 178, 218),
    status_bg: Color::Rgb(18, 20, 32),
    status_separator: Color::Rgb(116, 126, 156),
    status_brand_fg: Color::Rgb(16, 18, 26),
    status_brand_bg: Color::Rgb(105, 178, 218),
    status_filename_fg: Color::Rgb(162, 192, 222),
    status_filename_bg: Color::Rgb(24, 28, 44),
    status_watch_fg: Color::Rgb(95, 200, 148),
    status_watch_bg: Color::Rgb(18, 30, 24),
    status_reloaded_fg: Color::Rgb(16, 18, 26),
    status_reloaded_bg: Color::Rgb(95, 200, 148),
    status_search_fg: Color::Rgb(240, 210, 95),
    status_search_bg: Color::Rgb(36, 32, 16),
    status_success_fg: Color::Rgb(120, 210, 170),
    status_success_bg: Color::Rgb(18, 30, 24),
    status_warning_fg: Color::Rgb(240, 200, 60),
    status_error_fg: Color::Rgb(218, 95, 95),
    status_error_bg: Color::Rgb(42, 18, 18),
    status_shortcut_fg: Color::Rgb(58, 68, 98),
    status_percent_fg: Color::Rgb(105, 178, 218),
    toc_header_fg: Color::Rgb(88, 88, 96),
    toc_active_bg: Color::Rgb(42, 40, 46),
    toc_inactive_bg: Color::Rgb(18, 18, 22),
    toc_accent: Color::Rgb(123, 109, 255),
    toc_index_inactive: Color::Rgb(60, 60, 66),
    toc_primary_active: Color::Rgb(224, 224, 228),
    toc_primary_inactive: Color::Rgb(136, 136, 142),
    toc_secondary_inactive: Color::Rgb(62, 62, 68),
    toc_secondary_text_active: Color::Rgb(224, 224, 228),
    toc_secondary_text_inactive: Color::Rgb(102, 102, 108),
};

const BASE_LIGHT_MARKDOWN: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(232, 223, 164),
    code_gutter: Color::Rgb(132, 148, 164),
    blockquote_marker: Color::Rgb(124, 134, 184),
    list_level_1: Color::Rgb(48, 140, 98),
    list_level_2: Color::Rgb(90, 118, 188),
    list_level_3: Color::Rgb(128, 132, 148),
    ordered_list: Color::Rgb(48, 140, 98),
    table_border: Color::Rgb(138, 154, 172),
    table_separator: Color::Rgb(158, 170, 184),
    table_header: Color::Rgb(58, 108, 168),
    table_cell: Color::Rgb(58, 68, 78),
    heading_1: Color::Rgb(58, 108, 168),
    heading_2: Color::Rgb(48, 140, 98),
    heading_3: Color::Rgb(176, 128, 48),
    heading_other: Color::Rgb(108, 116, 126),
    heading_underline: Color::Rgb(160, 176, 194),
    code_frame: Color::Rgb(132, 148, 164),
    code_label: Color::Rgb(92, 116, 140),
    inline_code_fg: Color::Rgb(170, 108, 76),
    inline_code_bg: Color::Rgb(232, 226, 222),
    rule: Color::Rgb(180, 192, 204),
    link_icon: Color::Rgb(62, 124, 188),
    link_text: Color::Rgb(62, 124, 188),
    blockquote_text: Color::Rgb(114, 116, 158),
    text: Color::Rgb(58, 68, 78),
    strong_text: Color::Rgb(26, 32, 40),
    latex_inline_fg: Color::Rgb(128, 68, 148),
    latex_inline_bg: Color::Rgb(236, 226, 240),
    latex_block_fg: Color::Rgb(108, 58, 128),
};

const BASE_DARK_MARKDOWN: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(72, 62, 16),
    code_gutter: Color::Rgb(40, 48, 68),
    blockquote_marker: Color::Rgb(75, 80, 148),
    list_level_1: Color::Rgb(95, 200, 148),
    list_level_2: Color::Rgb(138, 155, 200),
    list_level_3: Color::Rgb(168, 168, 185),
    ordered_list: Color::Rgb(95, 200, 148),
    table_border: Color::Rgb(65, 75, 108),
    table_separator: Color::Rgb(55, 65, 95),
    table_header: Color::Rgb(140, 190, 255),
    table_cell: Color::Rgb(205, 208, 218),
    heading_1: Color::Rgb(140, 190, 255),
    heading_2: Color::Rgb(120, 210, 170),
    heading_3: Color::Rgb(210, 180, 120),
    heading_other: Color::Rgb(180, 180, 190),
    heading_underline: Color::Rgb(40, 50, 75),
    code_frame: Color::Rgb(40, 48, 68),
    code_label: Color::Rgb(95, 110, 145),
    inline_code_fg: Color::Rgb(220, 150, 118),
    inline_code_bg: Color::Rgb(38, 32, 31),
    rule: Color::Rgb(48, 56, 76),
    link_icon: Color::Rgb(85, 148, 235),
    link_text: Color::Rgb(88, 152, 238),
    blockquote_text: Color::Rgb(148, 148, 195),
    text: Color::Rgb(208, 210, 218),
    strong_text: Color::Rgb(245, 245, 255),
    latex_inline_fg: Color::Rgb(200, 160, 225),
    latex_inline_bg: Color::Rgb(38, 28, 48),
    latex_block_fg: Color::Rgb(195, 155, 220),
};

pub(crate) const ARCTIC_THEME: AppTheme = AppTheme {
    syntax_theme_name: "base16-ocean.light",
    ui: BASE_LIGHT_UI,
    markdown: BASE_LIGHT_MARKDOWN,
};

pub(crate) const FOREST_THEME: AppTheme = AppTheme {
    syntax_theme_name: "InspiredGitHub",
    ui: UiTheme {
        toc_bg: Color::Rgb(16, 22, 18),
        toc_border: Color::Rgb(50, 66, 54),
        content_bg: Color::Rgb(19, 26, 22),
        scrollbar_hover: Color::Rgb(126, 198, 170),
        status_bg: Color::Rgb(18, 27, 24),
        status_separator: Color::Rgb(112, 141, 126),
        status_brand_fg: Color::Rgb(14, 21, 18),
        status_brand_bg: Color::Rgb(120, 198, 148),
        status_filename_fg: Color::Rgb(184, 214, 196),
        status_filename_bg: Color::Rgb(26, 40, 32),
        status_watch_fg: Color::Rgb(132, 214, 154),
        status_watch_bg: Color::Rgb(19, 36, 28),
        status_reloaded_fg: Color::Rgb(14, 21, 18),
        status_reloaded_bg: Color::Rgb(132, 214, 154),
        status_search_fg: Color::Rgb(236, 214, 123),
        status_search_bg: Color::Rgb(38, 34, 18),
        status_success_fg: Color::Rgb(120, 214, 170),
        status_success_bg: Color::Rgb(20, 32, 24),
        status_warning_fg: Color::Rgb(236, 214, 123),
        status_error_fg: Color::Rgb(224, 120, 120),
        status_error_bg: Color::Rgb(42, 20, 20),
        status_shortcut_fg: Color::Rgb(82, 104, 92),
        status_percent_fg: Color::Rgb(126, 198, 170),
        toc_header_fg: Color::Rgb(102, 118, 106),
        toc_active_bg: Color::Rgb(34, 46, 38),
        toc_inactive_bg: Color::Rgb(16, 22, 18),
        toc_accent: Color::Rgb(127, 179, 255),
        toc_index_inactive: Color::Rgb(70, 82, 72),
        toc_primary_active: Color::Rgb(228, 234, 228),
        toc_primary_inactive: Color::Rgb(146, 156, 148),
        toc_secondary_inactive: Color::Rgb(76, 88, 80),
        toc_secondary_text_active: Color::Rgb(216, 224, 216),
        toc_secondary_text_inactive: Color::Rgb(112, 122, 114),
    },
    markdown: MarkdownTheme {
        search_highlight_bg: Color::Rgb(74, 78, 32),
        code_gutter: Color::Rgb(50, 66, 60),
        blockquote_marker: Color::Rgb(98, 124, 118),
        list_level_1: Color::Rgb(120, 198, 148),
        list_level_2: Color::Rgb(127, 179, 255),
        list_level_3: Color::Rgb(184, 190, 170),
        ordered_list: Color::Rgb(120, 198, 148),
        table_border: Color::Rgb(74, 92, 82),
        table_separator: Color::Rgb(60, 76, 68),
        table_header: Color::Rgb(148, 204, 255),
        table_cell: Color::Rgb(212, 218, 212),
        heading_1: Color::Rgb(148, 204, 255),
        heading_2: Color::Rgb(120, 214, 170),
        heading_3: Color::Rgb(224, 190, 126),
        heading_other: Color::Rgb(188, 194, 188),
        heading_underline: Color::Rgb(52, 68, 60),
        code_frame: Color::Rgb(50, 66, 60),
        code_label: Color::Rgb(128, 158, 142),
        inline_code_fg: Color::Rgb(224, 170, 132),
        inline_code_bg: Color::Rgb(42, 35, 31),
        rule: Color::Rgb(56, 70, 62),
        link_icon: Color::Rgb(102, 170, 255),
        link_text: Color::Rgb(110, 182, 255),
        blockquote_text: Color::Rgb(160, 168, 188),
        text: Color::Rgb(212, 218, 212),
        strong_text: Color::Rgb(246, 248, 246),
        latex_inline_fg: Color::Rgb(192, 162, 218),
        latex_inline_bg: Color::Rgb(34, 28, 42),
        latex_block_fg: Color::Rgb(188, 158, 214),
    },
};

pub(crate) const OCEAN_DARK_THEME: AppTheme = AppTheme {
    syntax_theme_name: "base16-ocean.dark",
    ui: BASE_DARK_UI,
    markdown: BASE_DARK_MARKDOWN,
};

pub(crate) const SOLARIZED_DARK_THEME: AppTheme = AppTheme {
    syntax_theme_name: "Solarized (dark)",
    ui: UiTheme {
        toc_bg: Color::Rgb(7, 54, 66),
        toc_border: Color::Rgb(88, 110, 117),
        content_bg: Color::Rgb(0, 43, 54),
        scrollbar_hover: Color::Rgb(42, 161, 152),
        status_bg: Color::Rgb(0, 43, 54),
        status_separator: Color::Rgb(101, 123, 131),
        status_brand_fg: Color::Rgb(0, 43, 54),
        status_brand_bg: Color::Rgb(42, 161, 152),
        status_filename_fg: Color::Rgb(147, 161, 161),
        status_filename_bg: Color::Rgb(7, 54, 66),
        status_watch_fg: Color::Rgb(133, 153, 0),
        status_watch_bg: Color::Rgb(18, 58, 34),
        status_reloaded_fg: Color::Rgb(0, 43, 54),
        status_reloaded_bg: Color::Rgb(133, 153, 0),
        status_search_fg: Color::Rgb(181, 137, 0),
        status_search_bg: Color::Rgb(32, 28, 0),
        status_success_fg: Color::Rgb(42, 161, 152),
        status_success_bg: Color::Rgb(0, 36, 32),
        status_warning_fg: Color::Rgb(181, 137, 0),
        status_error_fg: Color::Rgb(220, 50, 47),
        status_error_bg: Color::Rgb(48, 16, 15),
        status_shortcut_fg: Color::Rgb(88, 110, 117),
        status_percent_fg: Color::Rgb(42, 161, 152),
        toc_header_fg: Color::Rgb(101, 123, 131),
        toc_active_bg: Color::Rgb(17, 67, 80),
        toc_inactive_bg: Color::Rgb(7, 54, 66),
        toc_accent: Color::Rgb(38, 139, 210),
        toc_index_inactive: Color::Rgb(88, 110, 117),
        toc_primary_active: Color::Rgb(238, 232, 213),
        toc_primary_inactive: Color::Rgb(147, 161, 161),
        toc_secondary_inactive: Color::Rgb(88, 110, 117),
        toc_secondary_text_active: Color::Rgb(238, 232, 213),
        toc_secondary_text_inactive: Color::Rgb(131, 148, 150),
    },
    markdown: MarkdownTheme {
        search_highlight_bg: Color::Rgb(92, 74, 22),
        code_gutter: Color::Rgb(88, 110, 117),
        blockquote_marker: Color::Rgb(108, 113, 196),
        list_level_1: Color::Rgb(133, 153, 0),
        list_level_2: Color::Rgb(38, 139, 210),
        list_level_3: Color::Rgb(147, 161, 161),
        ordered_list: Color::Rgb(133, 153, 0),
        table_border: Color::Rgb(88, 110, 117),
        table_separator: Color::Rgb(101, 123, 131),
        table_header: Color::Rgb(38, 139, 210),
        table_cell: Color::Rgb(147, 161, 161),
        heading_1: Color::Rgb(38, 139, 210),
        heading_2: Color::Rgb(42, 161, 152),
        heading_3: Color::Rgb(181, 137, 0),
        heading_other: Color::Rgb(147, 161, 161),
        heading_underline: Color::Rgb(88, 110, 117),
        code_frame: Color::Rgb(88, 110, 117),
        code_label: Color::Rgb(131, 148, 150),
        inline_code_fg: Color::Rgb(190, 92, 48),
        inline_code_bg: Color::Rgb(22, 55, 60),
        rule: Color::Rgb(88, 110, 117),
        link_icon: Color::Rgb(38, 139, 210),
        link_text: Color::Rgb(38, 139, 210),
        blockquote_text: Color::Rgb(131, 148, 150),
        text: Color::Rgb(147, 161, 161),
        strong_text: Color::Rgb(238, 232, 213),
        latex_inline_fg: Color::Rgb(108, 113, 196),
        latex_inline_bg: Color::Rgb(14, 48, 58),
        latex_block_fg: Color::Rgb(108, 113, 196),
    },
};

pub(crate) const DEFAULT_PRESET: ThemePreset = ThemePreset::OceanDark;
pub(crate) const THEME_PRESETS: [ThemePreset; 4] = [
    ThemePreset::Arctic,
    ThemePreset::Forest,
    ThemePreset::OceanDark,
    ThemePreset::SolarizedDark,
];
static CURRENT_PRESET: AtomicU8 = AtomicU8::new(DEFAULT_PRESET as u8);

pub(crate) fn parse_theme_preset(name: &str) -> Option<ThemePreset> {
    match name {
        "arctic" => Some(ThemePreset::Arctic),
        "ocean" | "ocean-dark" | "dark" => Some(ThemePreset::OceanDark),
        "forest" => Some(ThemePreset::Forest),
        "solarized" | "solarized-dark" => Some(ThemePreset::SolarizedDark),
        _ => None,
    }
}

pub(crate) fn theme_preset_label(preset: ThemePreset) -> &'static str {
    match preset {
        ThemePreset::Arctic => "Arctic",
        ThemePreset::OceanDark => "Ocean Dark",
        ThemePreset::Forest => "Forest",
        ThemePreset::SolarizedDark => "Solarized Dark",
    }
}

pub(crate) fn theme_preset_index(preset: ThemePreset) -> usize {
    THEME_PRESETS
        .iter()
        .position(|candidate| *candidate == preset)
        .unwrap_or(0)
}

pub(crate) fn theme_by_preset(preset: ThemePreset) -> &'static AppTheme {
    match preset {
        ThemePreset::Arctic => &ARCTIC_THEME,
        ThemePreset::OceanDark => &OCEAN_DARK_THEME,
        ThemePreset::Forest => &FOREST_THEME,
        ThemePreset::SolarizedDark => &SOLARIZED_DARK_THEME,
    }
}

pub(crate) fn set_theme_preset(preset: ThemePreset) {
    CURRENT_PRESET.store(preset as u8, Ordering::Relaxed);
}

pub(crate) fn current_theme_preset() -> ThemePreset {
    match CURRENT_PRESET.load(Ordering::Relaxed) {
        0 => ThemePreset::Arctic,
        1 => ThemePreset::Forest,
        2 => ThemePreset::OceanDark,
        3 => ThemePreset::SolarizedDark,
        _ => DEFAULT_PRESET,
    }
}

pub(crate) fn app_theme() -> &'static AppTheme {
    theme_by_preset(current_theme_preset())
}

pub(crate) fn current_syntect_theme(themes: &ThemeSet) -> &Theme {
    &themes.themes[app_theme().syntax_theme_name]
}
