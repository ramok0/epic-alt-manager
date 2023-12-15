use egui::Align2;

use crate::gui::window::SubWindow;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct RuntimeSettings {
    pub advanced_mode:bool
}

pub struct SettingsWindow<'a> {
    runtime_settings:&'a mut RuntimeSettings
}

impl SubWindow for SettingsWindow<'_> {
    fn new(shared_data:crate::gui::window::WindowSharedData, window_descriptor:crate::gui::window::EWindow) -> Self where Self: Sized {
        todo!()
    }

    fn create_window<'a>(&self, ui:&egui::Ui) -> egui::Window<'a> where Self:Sized {
        egui::Window
        ::new("Configuration")
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .title_bar(true)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
    }

    fn render(&mut self, ctx:&egui::Context, ui:&mut egui::Ui) {
        ui.checkbox(&mut self.runtime_settings.advanced_mode, "Advanced mode");

                    if ui.button("Close").clicked() {
                        self.runtime_settings.advanced_mode = false;
                    }
    }

    fn close(&mut self) {
        todo!()
    }

    fn should_appear(&self) -> bool {
        todo!()
    }
}