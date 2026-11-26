use eframe::egui;
use rust_radio_gui::{RadioPanel, PANEL_HEIGHT, PANEL_WIDTH};

mod presets_file;

const APP_NAME: &str = "Rust Radio";

struct StandaloneRadio {
    radio: RadioPanel,
    stations: presets_file::PresetFile,
    #[cfg(feature = "nrsp-native")]
    presets: presets_file::PresetFile,
}

impl eframe::App for StandaloneRadio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.radio.show(ui));
        self.stations.poll(&mut self.radio);
        #[cfg(feature = "nrsp-native")]
        self.presets.poll(&mut self.radio);
    }
}

fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_inner_size([PANEL_WIDTH, PANEL_HEIGHT])
        .with_min_inner_size([PANEL_WIDTH, PANEL_HEIGHT]);
    // None asks eframe for its own icon. The explicit OS-default sentinel keeps
    // the macOS bundle's multi-resolution icon instead of replacing it at launch.
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
            radio.enable_input_discovery();
            let stations = presets_file::PresetFile::load_ip(&mut radio);
            #[cfg(feature = "nrsp-native")]
            let presets = presets_file::PresetFile::load(&mut radio);
            Ok(Box::new(StandaloneRadio {
                radio,
                stations,
                #[cfg(feature = "nrsp-native")]
                presets,
            }))
        }),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn macos_launch_preserves_bundle_icon_and_friendly_identity() {
        let viewport = native_options().viewport;
        // Some(empty) is intentional: None would restore eframe's default icon.
        assert_eq!(viewport.icon.as_deref(), Some(&egui::IconData::default()));
        assert_eq!(viewport.title.as_deref(), Some(APP_NAME));
        assert_eq!(
            viewport.inner_size,
            Some(egui::vec2(PANEL_WIDTH, PANEL_HEIGHT))
        );
        assert_eq!(viewport.min_inner_size, viewport.inner_size);
    }
}
