use super::{theme, RadioPanel, Segment};
use eframe::egui;
use rust_radio_core::directory::curated_stations;

const STATION_MENU_MAX_ROW_WIDTH: f32 = 340.0;
#[cfg(test)]
const PREVIOUS_STATION_MENU_ROW_WIDTH: f32 = 295.0;

impl RadioPanel {
    pub(super) fn station_selector(&mut self, ui: &mut egui::Ui) {
        let running = self.engine.is_some();
        let enabled = self.segment == Segment::Ip || self.is_native_selected() || !running;
        let picker_width = (ui.available_width() - 102.0).max(180.0);
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.segment == Segment::Ip || self.is_native_selected(),
                        egui::Button::new("◀").min_size(egui::vec2(44.0, 44.0)),
                    )
                    .on_hover_text("Previous station / frequency")
                    .clicked()
                {
                    self.step_segment_source(-1);
                }
                if self.segment == Segment::Ip {
                    ui.allocate_ui_with_layout(
                        egui::vec2(picker_width, 44.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.ip_station_menu(ui),
                    );
                } else {
                    ui.allocate_ui_with_layout(
                        egui::vec2(picker_width, 44.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
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
                                    .width(ui.available_width().min(360.0))
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
                        },
                    );
                }
                if ui
                    .add_enabled(
                        self.segment == Segment::Ip,
                        egui::Button::new("▶").min_size(egui::vec2(44.0, 44.0)),
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
            .map(|station| station_menu_label(station, true))
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
            .width(ui.available_width().min(360.0))
            .height(170.0)
            .truncate()
            .selected_text(
                egui::RichText::new(self.ip_station_name())
                    .strong()
                    .size(16.0),
            )
            .show_ui(ui, |ui| {
                theme::apply(ui);
                let row_width = ui.available_width().min(STATION_MENU_MAX_ROW_WIDTH);
                ui.set_width(row_width);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                for (index, name) in names.iter().enumerate() {
                    let health = if index == selected
                        && self.engine.as_ref().is_some_and(|engine| {
                            engine.status().state
                                == rust_radio_audio::compat_controller::PlaybackState::Playing
                        }) {
                        "Playing now".to_owned()
                    } else {
                        curated_stations()
                            .get(index)
                            .map(|station| {
                                format!("Last silent check: {}", self.validation_label(station.id))
                            })
                            .unwrap_or_else(|| "Custom stream".into())
                    };
                    let label = format!("{} {}", if index == selected { ">" } else { " " }, name);
                    let row = ui
                        .add_sized(
                            [row_width, 28.0],
                            egui::Button::new(label)
                                .selected(index == cursor)
                                .truncate(),
                        )
                        .on_hover_text(if let Some(station) = curated_stations().get(index) {
                            format!(
                                "{}\nGenre: {}\n{}\nPublisher: {}",
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

fn station_menu_label(
    station: &rust_radio_core::directory::InternetStation,
    show_category: bool,
) -> String {
    if show_category {
        format!("{} · {}", station.name, station.category)
    } else {
        station.name.to_owned()
    }
}

#[cfg(test)]
mod label_ab_test {
    use super::*;

    #[test]
    fn name_plus_category_rows_fit_without_truncation() {
        let context = egui::Context::default();
        let mut results = None;
        let mut button_horizontal_padding = 0.0;
        let _ = context.run(Default::default(), |ctx| {
            button_horizontal_padding = ctx.style().spacing.button_padding.x * 2.0;
            let current_row_text_width =
                PREVIOUS_STATION_MENU_ROW_WIDTH - button_horizontal_padding;
            let widened_row_text_width = STATION_MENU_MAX_ROW_WIDTH - button_horizontal_padding;
            results = Some(ctx.fonts(|fonts| {
                let font = egui::FontId::monospace(14.0);
                let mut category_current_fit = 0;
                let mut category_widened_fit = 0;
                let mut name_current_fit = 0;
                let mut name_widened_fit = 0;
                let mut saved_width = 0.0;
                for station in curated_stations() {
                    let category = format!("> {}", station_menu_label(station, true));
                    let name = format!("> {}", station_menu_label(station, false));
                    let category_width = fonts
                        .layout_no_wrap(category, font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    let name_width = fonts
                        .layout_no_wrap(name, font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    category_current_fit += usize::from(category_width <= current_row_text_width);
                    category_widened_fit += usize::from(category_width <= widened_row_text_width);
                    name_current_fit += usize::from(name_width <= current_row_text_width);
                    name_widened_fit += usize::from(name_width <= widened_row_text_width);
                    saved_width += category_width - name_width;
                }
                (
                    category_current_fit,
                    category_widened_fit,
                    name_current_fit,
                    name_widened_fit,
                    saved_width,
                )
            }));
        });
        let (
            category_current_fit,
            category_widened_fit,
            name_current_fit,
            name_widened_fit,
            saved_width,
        ) = results.unwrap();
        let station_count = curated_stations().len();
        eprintln!(
            "Station label A/B (monospace 14, {:.0}px button padding): A at {:.0}px {category_current_fit}/{station_count}, B at {:.0}px {name_current_fit}/{station_count}; A at {:.0}px {category_widened_fit}/{station_count}, B {name_widened_fit}/{station_count}; B saves {saved_width:.0}px total text width",
            button_horizontal_padding,
            PREVIOUS_STATION_MENU_ROW_WIDTH,
            PREVIOUS_STATION_MENU_ROW_WIDTH,
            STATION_MENU_MAX_ROW_WIDTH,
        );
        assert!(category_current_fit < name_current_fit);
        assert_eq!(name_current_fit, station_count);
        assert_eq!(category_widened_fit, station_count);
        assert_eq!(name_widened_fit, station_count);
        assert!(saved_width > 0.0);
    }
}
