use super::{theme, RadioPanel, Segment};
use eframe::egui;
use rust_radio_core::directory::curated_stations;

const STATION_MENU_MAX_ROW_WIDTH: f32 = 340.0;
const STATION_MENU_BUTTON_HORIZONTAL_PADDING: f32 = 10.0;
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
                ui.spacing_mut().button_padding.x = STATION_MENU_BUTTON_HORIZONTAL_PADDING;
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
        format!(
            "{} · {}",
            station.name,
            compact_station_category(station.category)
        )
    } else {
        station.name.to_owned()
    }
}

fn compact_station_category(category: &str) -> &str {
    match category {
        "CLASSICAL" => "CLASS",
        other => other,
    }
}

#[cfg(test)]
mod label_ab_test {
    use super::*;

    #[test]
    fn station_label_ab_compares_full_compact_and_name_only_variants() {
        let context = egui::Context::default();
        let mut results = None;
        let button_horizontal_padding = STATION_MENU_BUTTON_HORIZONTAL_PADDING * 2.0;
        let _ = context.run(Default::default(), |ctx| {
            let current_row_text_width =
                PREVIOUS_STATION_MENU_ROW_WIDTH - button_horizontal_padding;
            let widened_row_text_width = STATION_MENU_MAX_ROW_WIDTH - button_horizontal_padding;
            results = Some(ctx.fonts(|fonts| {
                let font = egui::FontId::monospace(14.0);
                let mut full_category_current_fit = 0;
                let mut full_category_widened_fit = 0;
                let mut compact_category_current_fit = 0;
                let mut compact_category_widened_fit = 0;
                let mut name_current_fit = 0;
                let mut name_widened_fit = 0;
                let mut compact_width_saved = 0.0;
                let mut current_overflows = Vec::new();
                for station in curated_stations() {
                    if station.category == "CLASSICAL" {
                        assert_eq!(compact_station_category(station.category), "CLASS");
                    } else {
                        assert_eq!(compact_station_category(station.category), station.category);
                    }
                    let full_category = format!("> {} · {}", station.name, station.category);
                    let compact_category = format!("> {}", station_menu_label(station, true));
                    let name = format!("> {}", station_menu_label(station, false));
                    let full_width = fonts
                        .layout_no_wrap(full_category, font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    let compact_width = fonts
                        .layout_no_wrap(compact_category, font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    let name_width = fonts
                        .layout_no_wrap(name, font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    full_category_current_fit += usize::from(full_width <= current_row_text_width);
                    if full_width > current_row_text_width {
                        current_overflows.push(station.id);
                    }
                    full_category_widened_fit += usize::from(full_width <= widened_row_text_width);
                    compact_category_current_fit +=
                        usize::from(compact_width <= current_row_text_width);
                    compact_category_widened_fit +=
                        usize::from(compact_width <= widened_row_text_width);
                    name_current_fit += usize::from(name_width <= current_row_text_width);
                    name_widened_fit += usize::from(name_width <= widened_row_text_width);
                    compact_width_saved += full_width - compact_width;
                }
                (
                    full_category_current_fit,
                    full_category_widened_fit,
                    compact_category_current_fit,
                    compact_category_widened_fit,
                    name_current_fit,
                    name_widened_fit,
                    compact_width_saved,
                    current_overflows,
                )
            }));
        });
        let (
            full_category_current_fit,
            full_category_widened_fit,
            compact_category_current_fit,
            compact_category_widened_fit,
            name_current_fit,
            name_widened_fit,
            compact_width_saved,
            current_overflows,
        ) = results.unwrap();
        let station_count = curated_stations().len();
        eprintln!(
            "Station label A/B/C (monospace 14, {:.0}px button padding): A full category at {:.0}px {full_category_current_fit}/{station_count}; B names-only {name_current_fit}/{station_count}; C compact category {compact_category_current_fit}/{station_count}; at {:.0}px A/B/C {full_category_widened_fit}/{station_count}/{compact_category_widened_fit}/{station_count}/{name_widened_fit}/{station_count}; C saves {compact_width_saved:.0}px vs A; A overflows {current_overflows:?}",
            button_horizontal_padding,
            PREVIOUS_STATION_MENU_ROW_WIDTH,
            STATION_MENU_MAX_ROW_WIDTH,
        );
        assert_eq!(
            full_category_current_fit + current_overflows.len(),
            station_count
        );
        assert_eq!(name_current_fit, station_count);
        assert_eq!(compact_category_current_fit, station_count);
        assert_eq!(full_category_widened_fit, station_count);
        assert_eq!(compact_category_widened_fit, station_count);
        assert_eq!(name_widened_fit, station_count);
        assert!(compact_width_saved > 0.0);
    }
}
