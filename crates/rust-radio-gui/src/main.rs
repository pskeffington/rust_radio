use eframe::egui;
use rust_radio_gui::{RadioPanel, PANEL_HEIGHT, PANEL_WIDTH};

mod station_file;

struct StandaloneRadio {
    radio: RadioPanel,
    stations: station_file::StationFile,
}

impl eframe::App for StandaloneRadio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.radio.show(ui));
        self.stations.poll(&mut self.radio);
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
            let mut radio = RadioPanel::new();
            let stations = station_file::StationFile::load(&mut radio);
            Ok(Box::new(StandaloneRadio { radio, stations }))
        }),
    )
}
