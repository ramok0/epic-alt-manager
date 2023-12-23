use egui::{Align2, FontId, RichText, Label};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CloneControlsData {
    pub clone_from: Option<String>,
    pub clone_to: String,
}

use crate::gui::{window::{WindowSharedData, SubWindow, EWindow, WindowDescriptor}, gui_helper::{centerer, add_button, EColor}, gui_constants::TEXT_COLOR, gui_workers_proc::clone_settings_proc};

pub struct CloneControlsWindow {
    information:CloneControlsData,
    shared_data:WindowSharedData,
    should_close:bool
}

impl SubWindow for CloneControlsWindow {
    fn new(shared_data:WindowSharedData, window_descriptor:WindowDescriptor) -> Self where Self: Sized {
        println!("created window");
        match window_descriptor.kind {
            EWindow::CloneSettings(info) => {
     
                Self {
                    information: info.clone(),
                    shared_data: shared_data,
                    should_close: false
                }
            },
            _ => panic!("Invalid window descriptor for CloneConfigurationWindow")
        }
    }

    fn create_window<'a>(&self, _ui:&egui::Ui) -> egui::Window<'a> where Self:Sized {
        egui::Window
        ::new("Clone controls")
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .title_bar(true)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
    }

    fn render(&mut self, ctx:&egui::Context, ui:&mut egui::Ui) {
        let font = FontId::new(14.0, egui::FontFamily::Name("Roboto".into()));

        self.create_window(ui).show(ctx, |ui| {
            centerer(ui, "_clone_controls", |ui| {
                ui.label(
                    RichText::new("I want to clone controls from ")
                        .font(font.clone())
                        .color(TEXT_COLOR)
                );
                egui::ComboBox
                    ::from_id_source("account selector")
                    .selected_text(
                        self.information.clone_from
                            .clone()
                            .unwrap_or(String::from("Select account"))
                    )
                    .show_ui(ui, |ui| {
                        let info = self.information.clone();
                        let mut selectable_string = |
                            ui: &mut egui::Ui,
                            element: String
                        | {
                            let from = info.clone_from.clone();
                            let currently_selected =
                               from.is_some() &&
                               from.unwrap() == element.clone();
                            let mut response = ui.selectable_label(
                                currently_selected,
                                RichText::new(element.clone())
                            );
                            if response.clicked() && !currently_selected {
                                //select account
                                self.information.clone_from = Some(element.clone());

                                response.mark_changed();
                            }
                            response
                        };

                        for account in self.shared_data.accounts.clone() {
                            if account != info.clone_to {
                                selectable_string(ui, account);
                            }
                        }
                    });

                ui.label(RichText::new("to").font(font.clone()).color(TEXT_COLOR));

                ui.add(
                    Label::new(
                        RichText::new(self.information.clone_to.clone())
                            .color(TEXT_COLOR)
                            .font(font.clone())
                            .strong()
                    )
                );
            });

            let is_account_selected = self.information.clone().clone_from.is_some();
            ui.add_space(5.0);
            centerer(ui, "_buttons", |ui| {
                ui.add_enabled_ui(is_account_selected, |ui| {
                    if add_button(ui, "Copy",  EColor::Primary).clicked() {
                        let info = self.information.clone();

                        let configuration_mtx = self.shared_data.configuration.clone();
                        let event_sender = self.shared_data.event_sender.clone();

                        let clone_from_username = info.clone_from.unwrap();
                        let clone_to_username = info.clone_to;

                         tokio::spawn(async move {
                            	let toast = 
                                    clone_settings_proc(configuration_mtx, clone_from_username, clone_to_username)
                                        .await
                                        .unwrap_or_else(|error| error.to_toast());

                                let _ = event_sender.send(crate::gui::window::EventKind::AddToast(toast)).await;
                         });
                    }
                });

                if add_button(ui, "Close", EColor::Delete).clicked() {
                    self.should_close = true;
                }
            });
        });
    }

    fn close(&mut self) {
        self.should_close = true;
    }

    fn should_appear(&self) -> bool {
        self.should_close == false
    }
}