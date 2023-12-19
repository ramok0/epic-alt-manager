use std::{sync::{ Arc, Mutex }, path::PathBuf, future};

use egui::{ Align2, Widget, FontSelection, TextStyle };
use egui_toast::{Toast, ToastKind, ToastOptions};

use crate::{
    gui::{window::{ SubWindow, WindowDescriptor }, gui_constants::TEXT_COLOR},
    launchers::{ Launchers, self },
    config::Configuration,
};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct RuntimeSettings {
    pub advanced_mode: bool,
}

pub struct SettingsWindow {
    pub runtime_settings: Arc<Mutex<RuntimeSettings>>,
    clone_launcher: Launchers,
    clone_legendary_path: String,
    should_close: bool,
    shared_data: crate::gui::window::WindowSharedData,
}

impl SubWindow for SettingsWindow {
    fn new(
        shared_data: crate::gui::window::WindowSharedData,
        window_descriptor: WindowDescriptor
    ) -> Self
        where Self: Sized
    {
        let lock = shared_data.configuration.clone().blocking_lock_owned();
         let current_launcher = &lock.launcher;
         let current_legendary_path = &lock.legendary_path;
        SettingsWindow {
            shared_data: shared_data,
            runtime_settings: window_descriptor.runtime_settings,
            should_close: false,
            clone_launcher: current_launcher.clone(),
            clone_legendary_path: current_legendary_path.to_owned(),
        }
    }

    fn create_window<'a>(&self, ui: &egui::Ui) -> egui::Window<'a> where Self: Sized {
        let font = FontSelection::default().resolve(ui.style());
        let text_size = ui.painter().layout_no_wrap(self.clone_legendary_path.clone(), font, TEXT_COLOR).size();

        egui::Window
            ::new("Configuration")
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(true)
            .min_width(text_size.x + 50.)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
    }

    fn render(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let font = FontSelection::default().resolve(ui.style());
        let text_size = ui.painter().layout_no_wrap(self.clone_legendary_path.clone(), font, TEXT_COLOR).size();

        self.create_window(ui).show(ctx, |ui| {
            let mut runtime_settings = self.runtime_settings.lock().unwrap();

            ui.checkbox(&mut runtime_settings.advanced_mode, "Advanced mode");

            egui::ComboBox
                ::from_label("Launcher")
                .selected_text(self.clone_launcher.to_string())
                .show_ui(ui, |ui| {
                    crate::launchers
                        ::launchers()
                        .iter()
                        .for_each(|launcher| {
                            if
                                ui
                                    .selectable_value(
                                        &mut self.clone_launcher,
                                        launcher.clone(),
                                        launcher.to_string()
                                    )
                                    .changed()
                            {
                                let mut configuration = self.shared_data.configuration.blocking_lock();
                                configuration.launcher = self.clone_launcher.clone();
                            }
                        });
                });

            let response = egui::TextEdit::singleline(&mut self.clone_legendary_path)
            .desired_width(text_size.x + 35.)
            .ui(ui)
            .on_hover_text("Legendary Configuration Path");

            if response.double_clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        if path.exists() && path.join("user.json").exists() {
                            let mut configuration = self.shared_data.configuration.blocking_lock();
                            let path_str = path.display().to_string();
                            configuration.legendary_path = path_str.clone();
                            self.clone_legendary_path = path_str;

                            let sender = self.shared_data.event_sender.clone();

                            tokio::spawn(async move {
                                let _ = sender.send(crate::gui::window::EventKind::AddToast(
                                    Toast {
                                        text: "Legendary Configuration Path updated".into(),
                                        kind: ToastKind::Success,
                                        options: ToastOptions::default()
                                            .duration_in_seconds(5.0)
                                            .show_progress(true)
                                            .show_icon(true),
                                    }
                                )).await;
                            });
                        }
                    }
                }

            if response.changed() {
                let path = PathBuf::from(&self.clone_legendary_path);
                if path.exists() && path.join("user.json").exists() {
                    let mut configuration = self.shared_data.configuration.blocking_lock();
                    configuration.legendary_path = self.clone_legendary_path.clone();

                    let sender = self.shared_data.event_sender.clone();

                    tokio::spawn(async move {
                        let _ = sender.send(crate::gui::window::EventKind::AddToast(
                            Toast {
                                text: "Legendary Configuration Path updated".into(),
                                kind: ToastKind::Success,
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true),
                            }
                        )).await;
                    });
                }
            }

            if ui.button("Close").clicked() {
                self.should_close = true;
            }
        });
    }

    fn close(&mut self) {
        self.should_close = true;
    }

    fn should_appear(&self) -> bool {
        !self.should_close
    }
}
