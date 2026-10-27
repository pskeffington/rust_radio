use eframe::egui;
use rust_radio_gui::{RadioPanel, PANEL_HEIGHT, PANEL_WIDTH};

struct StandaloneRadio {
    radio: RadioPanel,
}

impl eframe::App for StandaloneRadio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.radio.show(ui));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RUST RADIO / MICRO")
            .with_inner_size([PANEL_WIDTH, PANEL_HEIGHT])
            .with_min_inner_size([PANEL_WIDTH, PANEL_HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Radio",
        options,
        Box::new(|_cc| {
            Ok(Box::new(StandaloneRadio {
                radio: RadioPanel::new(),
            }))
        }),
    )
}
