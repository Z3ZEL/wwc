//! Every visual value comes from `assets/theme.json` (ARCHITECTURE §5.7).
//! UI code reads colors and sizes from `Theme`; never hard-code them.

use std::collections::BTreeMap;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Vec2};
use serde::{Deserialize, Deserializer};

const THEME_JSON: &str = include_str!("../../assets/theme.json");

/// Text style for panel section titles.
pub fn subheading() -> TextStyle {
    TextStyle::Name("Subheading".into())
}

macro_rules! color_struct {
    ($name:ident { $($field:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            $( #[serde(deserialize_with = "de_color")] pub $field: Color32, )*
        }
    };
}

color_struct!(Colors {
    background,
    surface,
    surface_alt,
    border,
    text,
    text_muted,
    text_disabled,
    link,
    primary,
    on_primary,
    accent,
    on_accent,
    button_bg,
    button_bg_hover,
    button_text,
    input_bg,
    input_border,
    input_border_focus,
    selection,
    success,
    warning,
    danger,
    info,
    top_bar_bg,
    top_bar_text,
    top_bar_text_muted,
    panel_bg,
    panel_header_bg,
    tag_bg,
    tag_text,
    tag_selected_bg,
    tag_selected_text,
    star_filled,
    star_empty,
    hidden_badge_bg,
    hidden_badge_text,
    toast_bg,
    toast_text,
    photo_viewer_backdrop,
    photo_viewer_bg,
    photo_viewer_text,
    photo_selected,
    spinner,
    spinner_track,
});

color_struct!(MapColors {
    marker,
    marker_own,
    marker_selected,
    marker_draft,
    cluster,
    cluster_text,
    marker_outline,
    label_bg,
    label_text,
    attribution_bg,
    attribution_text,
});

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub shape: Shape,
    pub layout: Layout,
    pub map: MapTheme,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Typography {
    pub font_family: String,
    pub font_family_heading: String,
    /// Font name → file in assets/fonts/ (`null` = egui's built-in font).
    pub fonts: BTreeMap<String, Option<String>>,
    pub size_heading: f32,
    pub size_subheading: f32,
    pub size_body: f32,
    pub size_button: f32,
    pub size_small: f32,
    pub size_monospace: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spacing {
    pub item_spacing: [f32; 2],
    pub button_padding: [f32; 2],
    pub panel_padding: f32,
    pub section_gap: f32,
    pub top_bar_padding: [f32; 2],
    /// Space between photo thumbnails.
    pub photo_gap: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub corner_radius: f32,
    pub corner_radius_tag: f32,
    pub border_width: f32,
    pub focus_border_width: f32,
    pub shadows: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub top_bar_height: f32,
    pub side_panel_width: f32,
    pub side_panel_min_width: f32,
    pub side_panel_max_width: f32,
    pub side_panel_collapsed_width: f32,
    pub panel_animation_ms: u32,
    pub narrow_breakpoint: f32,
    pub toast_duration_ms: u32,
    /// Photo thumbnails are drawn in a 4:3 box of this height.
    pub photo_thumb_height: f32,
    /// Gap between the full-page photo viewer and the window edges.
    pub photo_viewer_margin: f32,
    /// Height of the carousel thumbnails at the bottom of the viewer.
    pub photo_viewer_thumb_height: f32,
    /// Width of the top bar search field (wide / narrow screens).
    pub search_width: f32,
    pub search_width_narrow: f32,
    /// Two-handle range slider (tent capacity filter).
    pub range_slider_handle_radius: f32,
    pub range_slider_track_height: f32,
    /// Loading spinner diameter: next to a label, and for the main load of a panel.
    pub spinner_size_inline: f32,
    pub spinner_size_block: f32,
    pub spinner_stroke: f32,
    /// Time for one full turn.
    pub spinner_period_ms: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapTheme {
    pub default_center: LatLon,
    pub default_zoom: f64,
    pub max_zoom: f64,
    pub marker_radius: f32,
    pub marker_radius_selected: f32,
    pub marker_outline_width: f32,
    /// Markers closer than this on screen (points) are drawn as one cluster.
    pub cluster_distance: f32,
    /// Cluster circle radius: `cluster_radius` for 2 campsites, growing up to `cluster_radius_max`.
    pub cluster_radius: f32,
    pub cluster_radius_max: f32,
    /// From this zoom level on, every campsite gets its own marker.
    pub cluster_max_zoom: f64,
    /// Zoom levels added by a click on a cluster.
    pub cluster_zoom_step: f64,
    /// Minimum zoom when the map centers on a campsite (search result).
    pub focus_zoom: f64,
    pub colors: MapColors,
}

impl Theme {
    /// The theme embedded at build time. Its validity is checked by the unit tests below,
    /// so a parse failure here means the tests were skipped; fall back loudly.
    pub fn load() -> Self {
        match Self::parse(THEME_JSON) {
            Ok(t) => t,
            Err(e) => panic!("assets/theme.json is invalid: {e}"),
        }
    }

    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn radius(&self) -> CornerRadius {
        CornerRadius::same(self.shape.corner_radius.round().clamp(0.0, 255.0) as u8)
    }

    pub fn tag_radius(&self) -> CornerRadius {
        CornerRadius::same(self.shape.corner_radius_tag.round().clamp(0.0, 255.0) as u8)
    }

    pub fn panel_margin(&self) -> Margin {
        Margin::same(self.spacing.panel_padding.round().clamp(0.0, 127.0) as i8)
    }

    /// Map the theme onto egui's global style. Called once at startup.
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.all_styles_mut(|style| self.apply_to_style(style));
    }

    fn apply_to_style(&self, style: &mut egui::Style) {
        let c = &self.colors;
        let t = &self.typography;
        let (body, heading) = (family(&t.font_family), family(&t.font_family_heading));

        style.text_styles = [
            (TextStyle::Heading, FontId::new(t.size_heading, heading.clone())),
            (subheading(), FontId::new(t.size_subheading, heading)),
            (TextStyle::Body, FontId::new(t.size_body, body.clone())),
            (TextStyle::Button, FontId::new(t.size_button, body.clone())),
            (TextStyle::Small, FontId::new(t.size_small, body)),
            (TextStyle::Monospace, FontId::new(t.size_monospace, FontFamily::Monospace)),
        ]
        .into();

        style.animation_time = self.layout.panel_animation_ms as f32 / 1000.0;

        let s = &mut style.spacing;
        s.item_spacing = Vec2::from(self.spacing.item_spacing);
        s.button_padding = Vec2::from(self.spacing.button_padding);
        s.window_margin = self.panel_margin();
        s.menu_margin = Margin::same((self.spacing.item_spacing[0]).round() as i8);

        let border = Stroke::new(self.shape.border_width, c.border);
        let radius = self.radius();
        let v = &mut style.visuals;
        v.dark_mode = false;
        v.override_text_color = Some(c.text);
        v.weak_text_color = Some(c.text_muted);
        v.hyperlink_color = c.link;
        v.faint_bg_color = c.surface_alt;
        v.extreme_bg_color = c.input_bg;
        v.text_edit_bg_color = Some(c.input_bg);
        v.code_bg_color = c.surface_alt;
        v.warn_fg_color = c.warning;
        v.error_fg_color = c.danger;
        v.window_fill = c.surface;
        v.panel_fill = c.panel_bg;
        v.window_stroke = border;
        v.window_corner_radius = radius;
        v.menu_corner_radius = radius;
        v.selection.bg_fill = c.selection;
        v.selection.stroke = Stroke::new(self.shape.focus_border_width, c.input_border_focus);
        if !self.shape.shadows {
            v.window_shadow = Shadow::NONE;
            v.popup_shadow = Shadow::NONE;
        }

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = c.surface;
        w.noninteractive.weak_bg_fill = c.surface;
        w.noninteractive.bg_stroke = border;
        w.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
        w.noninteractive.corner_radius = radius;

        w.inactive.bg_fill = c.button_bg;
        w.inactive.weak_bg_fill = c.button_bg;
        w.inactive.bg_stroke = Stroke::new(self.shape.border_width, c.input_border);
        w.inactive.fg_stroke = Stroke::new(1.0, c.button_text);
        w.inactive.corner_radius = radius;

        for state in [&mut w.hovered, &mut w.open] {
            state.bg_fill = c.button_bg_hover;
            state.weak_bg_fill = c.button_bg_hover;
            state.bg_stroke = Stroke::new(self.shape.border_width, c.input_border_focus);
            state.fg_stroke = Stroke::new(1.0, c.button_text);
            state.corner_radius = radius;
            state.expansion = 0.0;
        }

        // egui also uses the "active" text color for `.strong()` text, so it stays dark;
        // primary (filled) buttons set their own colors.
        w.active.bg_fill = c.button_bg_hover;
        w.active.weak_bg_fill = c.button_bg_hover;
        w.active.bg_stroke = Stroke::new(self.shape.focus_border_width, c.input_border_focus);
        w.active.fg_stroke = Stroke::new(1.0, c.text);
        w.active.corner_radius = radius;
        w.active.expansion = 0.0;
    }
}

/// Only egui's built-in fonts are registered for now. A named font from
/// `typography.fonts` needs its bytes embedded in `install_fonts` first.
fn family(name: &str) -> FontFamily {
    if name != "default" {
        log::warn!("font {name:?} is not bundled yet; using the default font");
    }
    FontFamily::Proportional
}

fn de_color<'de, D: Deserializer<'de>>(d: D) -> Result<Color32, D::Error> {
    let s = String::deserialize(d)?;
    parse_hex(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid color {s:?}, expected #RRGGBB or #RRGGBBAA")))
}

/// `#RRGGBB` or `#RRGGBBAA`.
pub fn parse_hex(s: &str) -> Option<Color32> {
    let hex = s.strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

/// WCAG 2 contrast ratio between two opaque colors.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_json_parses() {
        let t = Theme::parse(THEME_JSON).expect("assets/theme.json must match Theme");
        assert!(t.layout.side_panel_min_width <= t.layout.side_panel_width);
        assert!(t.layout.side_panel_width <= t.layout.side_panel_max_width);
        assert!(t.map.default_zoom <= t.map.max_zoom);
        assert!(t.map.cluster_radius <= t.map.cluster_radius_max);
        assert!(t.map.focus_zoom <= t.map.max_zoom && t.map.cluster_max_zoom <= t.map.max_zoom);
        assert!(t.layout.spinner_size_inline > 0.0 && t.layout.spinner_size_inline <= t.layout.spinner_size_block);
        assert!(t.layout.spinner_stroke > 0.0 && t.layout.spinner_period_ms > 0);
    }

    #[test]
    fn text_is_readable() {
        let c = Theme::parse(THEME_JSON).expect("theme parses").colors;
        for (name, fg, bg) in [
            ("text/background", c.text, c.background),
            ("text/surface", c.text, c.surface),
            ("text_muted/surface", c.text_muted, c.surface),
            ("on_primary/primary", c.on_primary, c.primary),
            ("top_bar_text/top_bar_bg", c.top_bar_text, c.top_bar_bg),
            ("tag_text/tag_bg", c.tag_text, c.tag_bg),
            ("toast_text/toast_bg", c.toast_text, c.toast_bg),
            ("photo_viewer_text/photo_viewer_bg", c.photo_viewer_text, c.photo_viewer_bg),
            ("tag_selected_text/tag_selected_bg", c.tag_selected_text, c.tag_selected_bg),
        ] {
            let ratio = contrast_ratio(fg, bg);
            assert!(ratio >= 4.5, "{name} contrast is {ratio:.2}, needs >= 4.5");
        }
        let m = Theme::parse(THEME_JSON).expect("theme parses").map.colors;
        let ratio = contrast_ratio(m.cluster_text, m.cluster);
        assert!(ratio >= 4.5, "map cluster_text/cluster contrast is {ratio:.2}, needs >= 4.5");
    }

    #[test]
    fn hex_parsing() {
        assert_eq!(parse_hex("#FF0000"), Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(parse_hex("#00000080"), Some(Color32::from_rgba_unmultiplied(0, 0, 0, 128)));
        assert_eq!(parse_hex("FF0000"), None);
        assert_eq!(parse_hex("#FFF"), None);
        assert_eq!(parse_hex("#GG0000"), None);
    }
}
