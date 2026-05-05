use ratatui::style::Color;
use serde::{
    de::{Error as DeError, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::RwLock,
};
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppTheme {
    pub(crate) syntax_theme_name: Cow<'static, str>,
    pub(crate) ui: UiTheme,
    pub(crate) markdown: MarkdownTheme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub(crate) heading_4: Color,
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
    pub(crate) mermaid_keyword: Color,
    pub(crate) mermaid_arrow: Color,
    pub(crate) mermaid_label: Color,
    pub(crate) mermaid_block_fg: Color,
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
    heading_4: Color::Rgb(58, 84, 110),
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
    mermaid_keyword: Color::Rgb(0, 128, 128),
    mermaid_arrow: Color::Rgb(90, 120, 150),
    mermaid_label: Color::Rgb(28, 140, 120),
    mermaid_block_fg: Color::Rgb(68, 108, 118),
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
    heading_4: Color::Rgb(162, 192, 222),
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
    mermaid_keyword: Color::Rgb(80, 200, 200),
    mermaid_arrow: Color::Rgb(120, 160, 200),
    mermaid_label: Color::Rgb(100, 210, 180),
    mermaid_block_fg: Color::Rgb(160, 190, 200),
};

pub(crate) const ARCTIC_THEME: AppTheme = AppTheme {
    syntax_theme_name: Cow::Borrowed("base16-ocean.light"),
    ui: BASE_LIGHT_UI,
    markdown: BASE_LIGHT_MARKDOWN,
};

pub(crate) const FOREST_THEME: AppTheme = AppTheme {
    syntax_theme_name: Cow::Borrowed("InspiredGitHub"),
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
        heading_4: Color::Rgb(184, 214, 196),
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
        mermaid_keyword: Color::Rgb(80, 190, 170),
        mermaid_arrow: Color::Rgb(100, 170, 200),
        mermaid_label: Color::Rgb(90, 200, 160),
        mermaid_block_fg: Color::Rgb(150, 185, 170),
    },
};

pub(crate) const OCEAN_DARK_THEME: AppTheme = AppTheme {
    syntax_theme_name: Cow::Borrowed("base16-ocean.dark"),
    ui: BASE_DARK_UI,
    markdown: BASE_DARK_MARKDOWN,
};

pub(crate) const SOLARIZED_DARK_THEME: AppTheme = AppTheme {
    syntax_theme_name: Cow::Borrowed("Solarized (dark)"),
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
        heading_4: Color::Rgb(147, 161, 161),
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
        mermaid_keyword: Color::Rgb(42, 161, 152),
        mermaid_arrow: Color::Rgb(38, 139, 210),
        mermaid_label: Color::Rgb(88, 182, 172),
        mermaid_block_fg: Color::Rgb(131, 148, 150),
    },
};

pub(crate) const DEFAULT_PRESET: ThemePreset = ThemePreset::OceanDark;
pub(crate) const THEME_PRESETS: [ThemePreset; 4] = [
    ThemePreset::Arctic,
    ThemePreset::Forest,
    ThemePreset::OceanDark,
    ThemePreset::SolarizedDark,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CustomTheme {
    pub(crate) name: String,
    pub(crate) base_preset: ThemePreset,
    pub(crate) theme: AppTheme,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ThemeSelection {
    Preset(ThemePreset),
    Custom(Box<CustomTheme>),
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self::Preset(DEFAULT_PRESET)
    }
}

impl ThemeSelection {
    pub(crate) fn as_preset(&self) -> Option<ThemePreset> {
        match self {
            Self::Preset(preset) => Some(*preset),
            Self::Custom(_) => None,
        }
    }

    pub(crate) fn preset_hint(&self) -> ThemePreset {
        match self {
            Self::Preset(preset) => *preset,
            Self::Custom(custom) => custom.base_preset,
        }
    }

    pub(crate) fn app_theme(&self) -> AppTheme {
        match self {
            Self::Preset(preset) => theme_by_preset(*preset).clone(),
            Self::Custom(custom) => custom.theme.clone(),
        }
    }

    pub(crate) fn syntax_theme_name(&self) -> &str {
        match self {
            Self::Preset(preset) => theme_by_preset(*preset).syntax_theme_name.as_ref(),
            Self::Custom(custom) => custom.theme.syntax_theme_name.as_ref(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThemeColor(pub(crate) Color);

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = ThemeColor;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a color name, hex color string, rgb(...) string, or [r, g, b]")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                parse_theme_color(value)
                    .map(ThemeColor)
                    .ok_or_else(|| E::custom(format!("invalid theme color: {value}")))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                self.visit_str(&value)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let r = read_rgb_component(&mut seq, "red")?;
                let g = read_rgb_component(&mut seq, "green")?;
                let b = read_rgb_component(&mut seq, "blue")?;
                if seq.next_element::<u16>()?.is_some() {
                    return Err(A::Error::custom(
                        "theme RGB array must contain exactly 3 values",
                    ));
                }
                Ok(ThemeColor(Color::Rgb(r, g, b)))
            }
        }

        deserializer.deserialize_any(ColorVisitor)
    }
}

fn read_rgb_component<'de, A>(seq: &mut A, name: &str) -> Result<u8, A::Error>
where
    A: SeqAccess<'de>,
{
    let value = seq
        .next_element::<u16>()?
        .ok_or_else(|| A::Error::custom(format!("missing {name} theme color component")))?;
    u8::try_from(value)
        .map_err(|_| A::Error::custom(format!("{name} theme color component out of range")))
}

macro_rules! theme_overrides {
    ($name:ident for $theme:ty { $($field:ident),+ $(,)? }) => {
        #[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
        #[serde(default)]
        pub(crate) struct $name {
            $(pub(crate) $field: Option<ThemeColor>,)+
        }

        impl $name {
            fn apply_to(&self, theme: &mut $theme) {
                $(
                    if let Some(color) = self.$field {
                        theme.$field = color.0;
                    }
                )+
            }
        }
    };
}

theme_overrides!(UiThemeOverrides for UiTheme {
    toc_bg,
    toc_border,
    content_bg,
    scrollbar_hover,
    status_bg,
    status_separator,
    status_brand_fg,
    status_brand_bg,
    status_filename_fg,
    status_filename_bg,
    status_watch_fg,
    status_watch_bg,
    status_reloaded_fg,
    status_reloaded_bg,
    status_search_fg,
    status_search_bg,
    status_success_fg,
    status_success_bg,
    status_warning_fg,
    status_error_fg,
    status_error_bg,
    status_shortcut_fg,
    status_percent_fg,
    toc_header_fg,
    toc_active_bg,
    toc_inactive_bg,
    toc_accent,
    toc_index_inactive,
    toc_primary_active,
    toc_primary_inactive,
    toc_secondary_inactive,
    toc_secondary_text_active,
    toc_secondary_text_inactive,
});

theme_overrides!(MarkdownThemeOverrides for MarkdownTheme {
    search_highlight_bg,
    code_gutter,
    blockquote_marker,
    list_level_1,
    list_level_2,
    list_level_3,
    ordered_list,
    table_border,
    table_separator,
    table_header,
    table_cell,
    heading_1,
    heading_2,
    heading_3,
    heading_4,
    heading_other,
    heading_underline,
    code_frame,
    code_label,
    inline_code_fg,
    inline_code_bg,
    rule,
    link_icon,
    link_text,
    blockquote_text,
    text,
    strong_text,
    latex_inline_fg,
    latex_inline_bg,
    latex_block_fg,
    mermaid_keyword,
    mermaid_arrow,
    mermaid_label,
    mermaid_block_fg,
});

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct CustomThemeConfig {
    pub(crate) base: Option<String>,
    pub(crate) syntax: Option<String>,
    pub(crate) ui: UiThemeOverrides,
    pub(crate) markdown: MarkdownThemeOverrides,
}

static CURRENT_THEME: RwLock<ThemeSelection> = RwLock::new(ThemeSelection::Preset(DEFAULT_PRESET));

pub(crate) fn parse_theme_preset(name: &str) -> Option<ThemePreset> {
    match name {
        "arctic" => Some(ThemePreset::Arctic),
        "ocean" | "ocean-dark" | "dark" => Some(ThemePreset::OceanDark),
        "forest" => Some(ThemePreset::Forest),
        "solarized" | "solarized-dark" => Some(ThemePreset::SolarizedDark),
        _ => None,
    }
}

pub(crate) fn parse_theme_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(color) = parse_hex_color(value) {
        return Some(color);
    }
    if let Some(color) = parse_rgb_color(value) {
        return Some(color);
    }

    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark-gray" | "dark-grey" => Some(Color::DarkGray),
        "light-red" => Some(Color::LightRed),
        "light-green" => Some(Color::LightGreen),
        "light-yellow" => Some(Color::LightYellow),
        "light-blue" => Some(Color::LightBlue),
        "light-magenta" => Some(Color::LightMagenta),
        "light-cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    match hex.len() {
        3 if hex.chars().all(|c| c.is_ascii_hexdigit()) => {
            let mut expanded = String::with_capacity(6);
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            parse_hex_color(&expanded)
        }
        6 if hex.chars().all(|c| c.is_ascii_hexdigit()) => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn parse_rgb_color(value: &str) -> Option<Color> {
    let value = value.trim();
    let inner = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut components = inner.split(',').map(str::trim);
    let r = components.next()?.parse::<u8>().ok()?;
    let g = components.next()?.parse::<u8>().ok()?;
    let b = components.next()?.parse::<u8>().ok()?;
    if components.next().is_some() {
        return None;
    }
    Some(Color::Rgb(r, g, b))
}

pub(crate) fn resolve_theme_selection(
    name: &str,
    custom_themes: &BTreeMap<String, CustomThemeConfig>,
    theme_file_base_dir: Option<&Path>,
) -> Result<ThemeSelection, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Theme name cannot be empty".to_string());
    }

    if let Some(preset) = parse_theme_preset(name) {
        return Ok(ThemeSelection::Preset(preset));
    }

    if let Some(custom_config) = custom_themes.get(name) {
        return custom_theme_selection(name, custom_config);
    }

    if looks_like_theme_file(name) {
        return resolve_theme_file_selection(name, theme_file_base_dir);
    }

    Err(format!("Unknown theme \"{name}\""))
}

fn custom_theme_selection(
    name: &str,
    custom_config: &CustomThemeConfig,
) -> Result<ThemeSelection, String> {
    let base_preset = match custom_config.base.as_deref() {
        Some(base) => parse_theme_preset(base)
            .ok_or_else(|| format!("Unknown base theme \"{base}\" for custom theme \"{name}\""))?,
        None => DEFAULT_PRESET,
    };

    let mut theme = theme_by_preset(base_preset).clone();
    if let Some(syntax) = custom_config.syntax.as_deref() {
        let syntax = syntax.trim();
        if syntax.is_empty() {
            return Err(format!("Empty syntax theme for custom theme \"{name}\""));
        }
        theme.syntax_theme_name = Cow::Owned(syntax.to_string());
    }
    custom_config.ui.apply_to(&mut theme.ui);
    custom_config.markdown.apply_to(&mut theme.markdown);

    Ok(ThemeSelection::Custom(Box::new(CustomTheme {
        name: name.to_string(),
        base_preset,
        theme,
    })))
}

fn looks_like_theme_file(name: &str) -> bool {
    name.ends_with(".toml") || name.contains('/') || name.contains('\\')
}

fn resolve_theme_file_selection(
    name: &str,
    theme_file_base_dir: Option<&Path>,
) -> Result<ThemeSelection, String> {
    let path = resolve_theme_file_path(name, theme_file_base_dir);
    let content = std::fs::read_to_string(&path)
        .map_err(|err| format!("Cannot read theme file \"{}\": {err}", path.display()))?;
    let custom_config = toml::from_str::<CustomThemeConfig>(&content)
        .map_err(|err| format!("Could not parse theme file \"{}\": {err}", path.display()))?;
    let theme_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(name);
    custom_theme_selection(theme_name, &custom_config)
}

fn resolve_theme_file_path(name: &str, theme_file_base_dir: Option<&Path>) -> PathBuf {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    theme_file_base_dir
        .map(|base_dir| base_dir.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

pub(crate) fn validate_theme_syntax(
    selection: &ThemeSelection,
    themes: &ThemeSet,
) -> Option<String> {
    let syntax_theme_name = selection.syntax_theme_name();
    (!themes.themes.contains_key(syntax_theme_name)).then(|| {
        format!("Unknown syntax theme \"{syntax_theme_name}\", using default syntax colors")
    })
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

pub(crate) fn set_theme_selection(selection: ThemeSelection) {
    *CURRENT_THEME.write().expect("theme state lock poisoned") = selection;
}

pub(crate) fn current_theme_selection() -> ThemeSelection {
    CURRENT_THEME
        .read()
        .expect("theme state lock poisoned")
        .clone()
}

pub(crate) fn set_theme_preset(preset: ThemePreset) {
    set_theme_selection(ThemeSelection::Preset(preset));
}

#[cfg(test)]
pub(crate) fn current_theme_preset() -> ThemePreset {
    current_theme_selection().preset_hint()
}

pub(crate) fn app_theme() -> AppTheme {
    current_theme_selection().app_theme()
}

pub(crate) fn current_syntect_theme(themes: &ThemeSet) -> &Theme {
    let theme = app_theme();
    themes
        .themes
        .get(theme.syntax_theme_name.as_ref())
        .or_else(|| {
            themes
                .themes
                .get(theme_by_preset(DEFAULT_PRESET).syntax_theme_name.as_ref())
        })
        .or_else(|| themes.themes.values().next())
        .expect("syntect theme set is empty")
}
