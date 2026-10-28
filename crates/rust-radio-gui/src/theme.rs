//! Local phosphor theme for the panel and its settings window.
use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle};

pub(super) const GREEN: Color32 = Color32::from_rgb(155, 255, 133);
pub(super) const SCREEN: Color32 = Color32::from_rgb(5, 19, 9);
pub(super) const ERROR: Color32 = Color32::from_rgb(218, 255, 160);
const SURFACE: Color32 = Color32::from_rgb(12, 37, 17);
const HOVER: Color32 = Color32::from_rgb(29, 66, 30);
pub(super) const MUTED: Color32 = Color32::from_rgb(101, 166, 99);
pub(super) const ACTION: Color32 = Color32::from_rgb(22, 64, 28);
pub(super) const BORDER: Color32 = Color32::from_rgb(76, 155, 76);

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
    visuals.window_fill = SCREEN;
    visuals.panel_fill = SCREEN;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(1);
    visuals.error_fg_color = ERROR;
    visuals.warn_fg_color = ERROR;
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
    let stroke = Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(85, 180, 85, 5));
    for line in 0..60 {
        let y = rect.top() + line as f32 * 6.0;
        if y >= rect.bottom() {
            break;
        }
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
    }
}

pub(super) fn input_button(
    ui: &mut egui::Ui,
    label: &str,
    available: bool,
    selected: bool,
) -> egui::Response {
    let color = if available {
        GREEN
    } else {
        Color32::from_rgb(64, 91, 68)
    };
    let fill = if available {
        Color32::from_rgb(20, 57, 24)
    } else {
        SCREEN
    };
    let response = ui.add(
        egui::Button::new(egui::RichText::new(label).monospace().strong().color(color))
            .fill(fill)
            .stroke(Stroke::NONE)
            .min_size(egui::vec2(0.0, 28.0)),
    );
    if available {
        ui.painter().rect_stroke(
            response.rect.expand(1.0),
            2.0,
            Stroke::new(2.0_f32, Color32::from_rgba_unmultiplied(100, 255, 120, 30)),
            egui::StrokeKind::Outside,
        );
    }
    if selected {
        ui.painter().line_segment(
            [response.rect.left_bottom(), response.rect.right_bottom()],
            Stroke::new(1.0_f32, color),
        );
    }
    response
}
