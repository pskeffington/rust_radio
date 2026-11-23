use super::{theme, RadioPanel};
use eframe::egui;

impl RadioPanel {
    pub(super) fn show_settings(&mut self, ctx: &egui::Context) -> Option<egui::Rect> {
        egui::Window::new(
            egui::RichText::new("IP RADIO / SETUP")
                .monospace()
                .color(theme::GREEN),
        )
        .id(egui::Id::new("rust_radio_ip_setup"))
        .frame(
            egui::Frame::new()
                .fill(theme::SCREEN)
                .stroke(egui::Stroke::new(1.0_f32, theme::GREEN))
                .inner_margin(egui::Margin::same(8)),
        )
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(380.0, 230.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            theme::apply(ui);
            self.show_ip_settings(ui);
        })
        .map(|response| response.response.rect)
    }
}
