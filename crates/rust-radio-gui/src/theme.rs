//! Local phosphor theme for the panel and its settings window.
use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle};

pub(super) const GREEN: Color32 = Color32::from_rgb(155, 255, 133);
pub(super) const SCREEN: Color32 = Color32::from_rgb(5, 19, 9);
const SURFACE: Color32 = Color32::from_rgb(12, 37, 17);
const HOVER: Color32 = Color32::from_rgb(29, 66, 30);
const BORDER: Color32 = Color32::from_rgb(76, 155, 76);

pub(super) fn apply(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    for (kind, size) in [
        (TextStyle::Body, 14.0),
        (TextStyle::Button, 14.0),
        (TextStyle::Small, 11.0),
        (TextStyle::Heading, 17.0),
    ] {
        style.text_styles.insert(kind, FontId::monospace(size));
    }
    let visuals = &mut style.visuals;
    visuals.override_text_color = Some(GREEN);
    visuals.extreme_bg_color = SCREEN;
    visuals.faint_bg_color = SURFACE;
    visuals.selection.bg_fill = HOVER;
    visuals.selection.stroke = Stroke::new(1.0_f32, GREEN);
    visuals.slider_trailing_fill = true;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = SURFACE;
        widget.weak_bg_fill = SURFACE;
        widget.bg_stroke = Stroke::new(1.0_f32, BORDER);
        widget.fg_stroke = Stroke::new(1.0_f32, GREEN);
        widget.corner_radius = CornerRadius::same(1);
    }
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.active.bg_fill = HOVER;
    visuals.widgets.active.weak_bg_fill = HOVER;
    visuals.widgets.open.bg_fill = HOVER;
    visuals.widgets.open.weak_bg_fill = HOVER;
}

pub(super) fn scanlines(ui: &egui::Ui) {
    let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let stroke = Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(85, 180, 85, 12));
    for line in 0..80 {
        let y = rect.top() + line as f32 * 4.0;
        if y >= rect.bottom() {
            break;
        }
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
    }
}
