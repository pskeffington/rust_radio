use super::{
    fm_presets::{self, FmPreset, PresetBank},
    RadioPanel,
};
use eframe::egui;
use rust_radio_audio::{
    compat_controller::PlaybackState,
    playback::{NativeFmDeemphasis, NativeRadioEngine},
};

impl RadioPanel {
    fn native_playing(&self) -> bool {
        !self.stopping
            && self
                .engine
                .as_ref()
                .is_some_and(|engine| engine.status().state == PlaybackState::Playing)
    }
    pub(super) fn show_native_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (index, text) in ["TUNE", "FM AUDIO", "PRESETS", "DEVICE"].iter().enumerate() {
                ui.selectable_value(&mut self.native_tab, index as u8, *text);
            }
            if ui.button("CLOSE").clicked() {
                self.settings_open = false;
            }
        });
        ui.separator();
        match self.native_tab {
            1 => self.fm_audio_controls(ui),
            2 => self.fm_preset_controls(ui),
            3 => self.native_device_controls(ui),
            _ => self.native_tuning_controls(ui),
        }
    }
    fn native_tuning_controls(&mut self, ui: &mut egui::Ui) {
        let mut apply = false;
        ui.horizontal(|ui| {
            ui.label("FREQUENCY");
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.native_frequency)
                    .hint_text("99.5 MHz")
                    .desired_width(220.0)
                    .char_limit(24),
            );
            apply |= edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.stopping, egui::Button::new("− STEP"))
                .clicked()
            {
                self.step_native_frequency(false);
            }
            egui::ComboBox::from_id_salt("rust_radio_fm_step")
                .width(95.0)
                .selected_text(format!("{} kHz", self.native_step / 1000))
                .show_ui(ui, |ui| {
                    for step in [1_000, 10_000, 50_000, 100_000, 200_000] {
                        ui.selectable_value(
                            &mut self.native_step,
                            step,
                            format!("{} kHz", step / 1000),
                        );
                    }
                });
            if ui
                .add_enabled(!self.stopping, egui::Button::new("+ STEP"))
                .clicked()
            {
                self.step_native_frequency(true);
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("rust_radio_native_mode")
                .width(70.0)
                .selected_text(self.native_mode.to_uppercase())
                .show_ui(ui, |ui| {
                    for (mode, bandwidth) in [
                        ("wfm", "200000"),
                        ("nfm", "12000"),
                        ("am", "10000"),
                        ("usb", "3000"),
                        ("lsb", "3000"),
                        ("cw", "500"),
                        ("sam", "10000"),
                    ] {
                        if ui
                            .selectable_label(self.native_mode == mode, mode.to_uppercase())
                            .clicked()
                        {
                            self.native_mode = mode.into();
                            self.native_bandwidth = bandwidth.into();
                        }
                    }
                });
            ui.label("RF BW Hz");
            ui.add(
                egui::TextEdit::singleline(&mut self.native_bandwidth)
                    .desired_width(110.0)
                    .char_limit(6),
            );
        });
        if ui
            .add_enabled(self.native_playing(), egui::Button::new("APPLY TUNING"))
            .clicked()
            || apply && self.native_playing()
        {
            self.apply_native_tuning();
        }
        if let Some(status) = self
            .engine
            .as_ref()
            .and_then(NativeRadioEngine::nrsp_native_status)
        {
            ui.label(format!(
                "Receiver: {}",
                fm_presets::format_frequency(status.center_frequency_hz)
            ));
        } else {
            ui.label("Set tuning, close setup, then press PLAY.");
        }
    }
    fn fm_audio_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.native_mode == "wfm" && !self.stopping, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.native_fm.stereo, "STEREO");
                ui.label("DE-EMPHASIS");
                egui::ComboBox::from_id_salt("rust_radio_fm_deemphasis")
                    .width(70.0)
                    .selected_text(format!("{} µs", self.native_fm.deemphasis as u32))
                    .show_ui(ui, |ui| {
                        for (mode, label) in [
                            (NativeFmDeemphasis::Us75, "75 µs"),
                            (NativeFmDeemphasis::Us50, "50 µs"),
                            (NativeFmDeemphasis::Off, "OFF"),
                        ] {
                            ui.selectable_value(&mut self.native_fm.deemphasis, mode, label);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.native_fm.filter_enabled, "FILTER");
                ui.checkbox(&mut self.native_fm.limiter, "LIMITER");
            });
            ui.horizontal(|ui| {
                ui.label("LOW Hz");
                ui.add(
                    egui::DragValue::new(&mut self.native_fm.low_cut_hz)
                        .range(0..=1000)
                        .speed(10),
                );
                ui.label("HIGH Hz");
                ui.add(
                    egui::DragValue::new(&mut self.native_fm.high_cut_hz)
                        .range(3000..=15000)
                        .speed(100),
                );
            });
            if ui
                .add_enabled(self.native_playing(), egui::Button::new("APPLY FM AUDIO"))
                .clicked()
            {
                self.note = match self
                    .engine
                    .as_ref()
                    .expect("playing engine")
                    .set_nrsp_native_fm(self.native_fm)
                {
                    Ok(()) => "FM AUDIO REQUESTED".into(),
                    Err(error) => error,
                };
            }
        });
        let status = self
            .engine
            .as_ref()
            .and_then(NativeRadioEngine::nrsp_native_status);
        ui.label(if self.native_mode != "wfm" {
            "Select WFM in TUNE for broadcast FM audio."
        } else if !self.native_fm.stereo {
            "FORCED MONO"
        } else {
            match status.and_then(|status| status.stereo_locked) {
                Some(true) => "STEREO LOCK",
                Some(false) => "NO STEREO LOCK",
                None => "STEREO LOCK NOT YET REPORTED",
            }
        });
        let applied = self
            .engine
            .as_ref()
            .and_then(NativeRadioEngine::nrsp_native_status)
            .and_then(|status| status.fm_settings);
        ui.label(if !self.native_playing() {
            "APPLIED ON NEXT PLAY"
        } else if applied == Some(self.native_fm) {
            "FM SETTINGS SENT"
        } else {
            "CHANGES NOT APPLIED"
        });
        ui.label("75 µs: US / 50 µs: Europe and many regions.");
    }
    fn fm_preset_controls(&mut self, ui: &mut egui::Ui) {
        let mut recall = None;
        let mut remove = None;
        egui::ScrollArea::vertical()
            .id_salt("rust_radio_fm_presets")
            .max_height(105.0)
            .show(ui, |ui| {
                for (index, preset) in self.native_presets.stations.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let label = format!(
                            "{} / {:.3} MHz",
                            preset.name,
                            preset.frequency_hz as f64 / 1_000_000.0
                        );
                        if ui
                            .add_sized([270.0, 22.0], egui::Button::new(&label).truncate())
                            .on_hover_text(&label)
                            .clicked()
                        {
                            recall = Some(index);
                        }
                        if ui.small_button("REMOVE").clicked() {
                            remove = Some(index);
                        }
                    });
                }
            });
        if let Some(index) = recall {
            self.recall_native_preset(index);
        }
        if let Some(index) = remove {
            self.native_presets.stations.remove(index);
            self.native_presets_dirty = true;
            self.note = "PRESET REMOVED / SAVING".into();
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.native_preset_name)
                    .hint_text("Preset name")
                    .desired_width(190.0)
                    .char_limit(32),
            );
            if ui.button("SAVE CURRENT").clicked() {
                self.save_native_preset();
            }
        });
        ui.label("Presets include tuning and FM audio settings.");
    }
    fn native_device_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("RECEIVER");
            ui.add_enabled(
                self.engine.is_none(),
                egui::TextEdit::singleline(&mut self.native_endpoint)
                    .hint_text("IP:9001")
                    .desired_width(200.0)
                    .char_limit(40),
            );
        });
        ui.add_enabled_ui(self.engine.is_none(), |ui| {
            egui::ComboBox::from_id_salt("rust_radio_native_found")
                .width(320.0)
                .selected_text("Detected receivers")
                .show_ui(ui, |ui| {
                    for device in &self.inputs.network {
                        ui.add_enabled_ui(device.ready, |ui| {
                            ui.selectable_value(
                                &mut self.native_endpoint,
                                device.id.clone(),
                                format!("{} / {}", device.label, device.id),
                            );
                        });
                    }
                });
            egui::ComboBox::from_id_salt("rust_radio_native_output")
                .width(320.0)
                .selected_text(
                    self.outputs
                        .iter()
                        .find(|device| device.id == self.output)
                        .map(|device| device.name.as_str())
                        .unwrap_or("System default output"),
                )
                .show_ui(ui, |ui| {
                    for device in &self.outputs {
                        ui.add_enabled_ui(device.available, |ui| {
                            ui.selectable_value(&mut self.output, device.id.clone(), &device.name);
                        });
                    }
                });
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.is_active(), egui::Button::new("SCAN INPUTS"))
                .clicked()
            {
                self.scan_inputs();
            }
            if ui.button("REFRESH OUTPUTS").clicked() {
                self.refresh_outputs();
            }
        });
        ui.label("Stop before changing receiver or output.");
    }
    pub(super) fn step_native_frequency(&mut self, up: bool) {
        let result = (|| {
            let current = fm_presets::frequency_hz(&self.native_frequency)?;
            let next = if up {
                current.checked_add(self.native_step)
            } else {
                current.checked_sub(self.native_step)
            }
            .ok_or("Frequency step outside range")?;
            if self.native_mode == "wfm" && !(64_000_000..=108_000_000).contains(&next) {
                return Err("FM stepping stays within 64–108 MHz".into());
            }
            fm_presets::frequency_hz(&next.to_string())?;
            self.native_frequency = fm_presets::format_frequency(next);
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) if self.native_playing() => self.apply_native_tuning(),
            Ok(()) => self.note = "TUNING READY / PRESS PLAY".into(),
            Err(error) => self.note = error,
        }
    }
    pub(super) fn recall_native_preset(&mut self, index: usize) {
        let Some(preset) = self.native_presets.stations.get(index).cloned() else {
            return;
        };
        let Ok(settings) = preset.settings() else {
            return;
        };
        self.native_frequency = fm_presets::format_frequency(preset.frequency_hz);
        self.native_bandwidth = preset.bandwidth_hz.to_string();
        self.native_mode = "wfm".into();
        self.native_fm = settings;
        self.native_preset_name = preset.name;
        if self.native_playing() {
            self.apply_native_tuning()
        } else {
            self.note = "PRESET READY / PRESS PLAY".into();
        }
    }
    pub(super) fn save_native_preset(&mut self) {
        let result = (|| {
            if self.native_mode != "wfm" {
                return Err("Select WFM before saving an FM preset".into());
            }
            let frequency = fm_presets::frequency_hz(&self.native_frequency)?;
            let bandwidth = self
                .native_bandwidth
                .parse()
                .map_err(|_| "Invalid RF bandwidth")?;
            let preset = FmPreset::new(
                self.native_preset_name.clone(),
                frequency,
                bandwidth,
                self.native_fm,
            )?;
            self.native_presets.save(preset)
        })();
        self.note = match result {
            Ok(()) => {
                self.native_presets_dirty = true;
                "SAVING PRESETS…".into()
            }
            Err(error) => error,
        };
    }
    /// Pure host-owned preference import. Invalid input leaves the current bank intact.
    pub fn load_fm_presets(&mut self, json: &str) -> Result<(), String> {
        self.native_presets = PresetBank::parse(json)?;
        Ok(())
    }
    /// Latest bounded preset update for the host to persist outside the UI thread.
    pub fn take_fm_presets_update(&mut self) -> Option<String> {
        if !self.native_presets_dirty {
            return None;
        }
        self.native_presets_dirty = false;
        Some(self.native_presets.json())
    }
    pub fn report_fm_presets_saved(&mut self, result: Result<(), String>) {
        self.note = match result {
            Ok(()) => "FM PRESETS SAVED".into(),
            Err(error) => format!("PRESET SAVE FAILED: {error}"),
        };
    }
}
