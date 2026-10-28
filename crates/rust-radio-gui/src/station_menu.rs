use super::{theme, RadioPanel, Segment};
use eframe::egui;
use rust_radio_core::directory::curated_stations;

impl RadioPanel {
    pub(super) fn station_selector(&mut self, ui: &mut egui::Ui) {
        let running = self.engine.is_some();
        let enabled = self.segment == Segment::Ip || self.is_native_selected() || !running;
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.segment == Segment::Ip || self.is_native_selected(),
                        egui::Button::new("◀").min_size(egui::vec2(43.0, 40.0)),
                    )
                    .on_hover_text("Previous station / frequency")
                    .clicked()
                {
                    self.step_segment_source(-1);
                }
                if self.segment == Segment::Ip {
                    ui.allocate_ui_with_layout(
                        egui::vec2(280.0, 40.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.ip_station_menu(ui),
                    );
                } else {
                    ui.add_enabled_ui(!running, |ui| {
                        #[cfg(feature = "nrsp-native")]
                        let native = self.native_frequency.clone();
                        #[cfg(not(feature = "nrsp-native"))]
                        let native = "nRSP-ST / Direct".to_owned();
                        let label = if self.is_native_selected() {
                            &native
                        } else if self.is_usb_selected() {
                            "USB SDR / mono FM"
                        } else {
                            "nSDR / receiver"
                        };
                        egui::ComboBox::from_id_salt("rust_radio_receiver")
                            .width(260.0)
                            .truncate()
                            .selected_text(label)
                            .show_ui(ui, |ui| {
                                theme::apply(ui);
                                #[cfg(feature = "nrsp-native")]
                                if self.segment == Segment::Network
                                    && ui
                                        .selectable_label(
                                            self.is_native_selected(),
                                            "nRSP-ST / Direct",
                                        )
                                        .clicked()
                                {
                                    self.select_source(
                                        curated_stations().len()
                                            + usize::from(cfg!(feature = "sdrconnect"))
                                            + usize::from(cfg!(feature = "soapy-sdr"))
                                            + 1,
                                    );
                                }
                                #[cfg(feature = "sdrconnect")]
                                if self.segment == Segment::Network
                                    && ui
                                        .selectable_label(
                                            self.is_sdr_selected(),
                                            "nRSP-ST / SDRconnect",
                                        )
                                        .clicked()
                                {
                                    self.select_source(curated_stations().len());
                                }
                                #[cfg(feature = "soapy-sdr")]
                                if self.segment == Segment::Usb
                                    && ui
                                        .selectable_label(
                                            self.is_usb_selected(),
                                            "USB SDR / mono FM",
                                        )
                                        .clicked()
                                {
                                    self.select_source(
                                        curated_stations().len()
                                            + usize::from(cfg!(feature = "sdrconnect")),
                                    );
                                }
                            });
                    });
                }
                if ui
                    .add_enabled(
                        self.segment == Segment::Ip,
                        egui::Button::new("▶").min_size(egui::vec2(43.0, 40.0)),
                    )
                    .on_hover_text("Next station")
                    .clicked()
                {
                    self.step_segment_source(1);
                }
            });
        });
    }
    pub(super) fn ip_station_menu(&mut self, ui: &mut egui::Ui) {
        let selected = self.ip_station_index();
        let count = self.ip_station_count();
        let id = ui.make_persistent_id("rust_radio_station");
        let cursor_id = id.with("cursor");
        let open = egui::ComboBox::is_open(ui.ctx(), id);
        let mut cursor = if open {
            ui.data_mut(|data| data.get_temp::<usize>(cursor_id))
                .unwrap_or(selected)
        } else {
            selected
        };
        let mut choice = None;
        let names: Vec<String> = curated_stations()
            .iter()
            .map(|station| format!("{} · {}", station.name, station.category))
            .chain(self.ip_stations.iter().map(|station| station.name.clone()))
            .chain(std::iter::once("Custom public / LAN stream".into()))
            .collect();
        let mut moved = false;
        if open {
            let action = ui.input_mut(|input| {
                [
                    egui::Key::ArrowDown,
                    egui::Key::ArrowUp,
                    egui::Key::Home,
                    egui::Key::End,
                    egui::Key::Enter,
                ]
                .into_iter()
                .find(|&key| input.consume_key(egui::Modifiers::NONE, key))
            });
            match action {
                Some(egui::Key::ArrowDown) => {
                    cursor = (cursor + 1) % count;
                    moved = true;
                }
                Some(egui::Key::ArrowUp) => {
                    cursor = (cursor + count - 1) % count;
                    moved = true;
                }
                Some(egui::Key::Home) => {
                    cursor = 0;
                    moved = true;
                }
                Some(egui::Key::End) => {
                    cursor = count - 1;
                    moved = true;
                }
                Some(egui::Key::Enter) => choice = Some(cursor),
                _ => {}
            }
        }
        let response = egui::ComboBox::from_id_salt("rust_radio_station")
            .width(260.0)
            .height(170.0)
            .truncate()
            .selected_text(self.ip_station_name())
            .show_ui(ui, |ui| {
                theme::apply(ui);
                ui.set_width(295.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                for (index, name) in names.iter().enumerate() {
                    let health = curated_stations()
                        .get(index)
                        .map(|station| self.validation_label(station.id))
                        .unwrap_or("CUSTOM");
                    let label = format!("{} {}", if index == selected { ">" } else { " " }, name);
                    let row = ui
                        .add_sized(
                            [295.0, 28.0],
                            egui::Button::new(label)
                                .selected(index == cursor)
                                .truncate(),
                        )
                        .on_hover_text(if let Some(station) = curated_stations().get(index) {
                            format!(
                                "{}\nGenre: {}\nFeed: {}\nPublisher: {}",
                                station.name,
                                station.genre,
                                health,
                                station.publisher_reference_url
                            )
                        } else {
                            name.clone()
                        });
                    if row.clicked() {
                        choice = Some(index);
                    }
                    if moved && index == cursor {
                        row.scroll_to_me(Some(egui::Align::Center));
                    }
                }
                if choice.is_some() {
                    ui.close_menu();
                }
            })
            .response
            .on_hover_text(format!(
                "{}\nTab to focus; Enter opens; arrows navigate; Enter selects; Escape closes",
                names[selected]
            ));
        ui.data_mut(|data| data.insert_temp(cursor_id, cursor));
        if !open && response.has_focus() {
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight))
            {
                choice = Some((selected + 1) % count);
            }
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft))
            {
                choice = Some((selected + count - 1) % count);
            }
        }
        if let Some(index) = choice {
            ui.memory_mut(|memory| memory.close_popup());
            if index != selected {
                self.select_ip_station(index);
                self.ip_selection_changed();
            }
        }
    }
}
