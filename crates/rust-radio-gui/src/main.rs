use eframe::egui;
use rust_radio_gui::{RadioPanel, PANEL_HEIGHT, PANEL_WIDTH};

mod station_file;

const APP_NAME: &str = "Rust Radio";

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

fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("RUST RADIO / MICRO")
        .with_inner_size([PANEL_WIDTH, PANEL_HEIGHT])
        .with_min_inner_size([PANEL_WIDTH, PANEL_HEIGHT]);
    // An unset icon makes eframe supply its white "e". This empty OS-default
    // sentinel leaves the designed multi-resolution macOS bundle icon intact.
    #[cfg(target_os = "macos")]
    let viewport = viewport.with_icon(egui::IconData::default());
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        APP_NAME,
        native_options(),
        Box::new(|_cc| {
            let mut radio = RadioPanel::new();
            let stations = station_file::StationFile::load(&mut radio);
            Ok(Box::new(StandaloneRadio { radio, stations }))
        }),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn macos_launch_preserves_bundle_icon_and_identity() {
        let viewport = native_options().viewport;
        assert_eq!(viewport.icon.as_deref(), Some(&egui::IconData::default()));
        assert_eq!(viewport.title.as_deref(), Some("RUST RADIO / MICRO"));
        assert_eq!(
            viewport.inner_size,
            Some(egui::vec2(PANEL_WIDTH, PANEL_HEIGHT))
        );
        assert_eq!(viewport.min_inner_size, viewport.inner_size);
        assert_eq!(APP_NAME, "Rust Radio");
    }
}
