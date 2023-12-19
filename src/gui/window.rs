use std::sync::Arc;

use egui_toast::Toast;
use tokio::sync::{mpsc::Sender, Mutex, mpsc::Receiver};

use super::windows::{add_account::AddAccountWindow, clone_configuration::{CloneControlsData, CloneControlsWindow}, settings::RuntimeSettings};

#[derive(PartialEq, Clone, Hash, Eq, Debug)]
pub enum EWindow {
    AddAccount,
    CloneSettings(CloneControlsData),
    Settings
}

#[derive(Clone)]
pub struct WindowDescriptor {
    pub kind:EWindow,
    pub runtime_settings:Arc<std::sync::Mutex<RuntimeSettings>>
}


pub struct WindowManager {
    pub current_window:Option<(EWindow, Box<dyn SubWindow>)>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self { current_window: None }
    }

    pub fn render(&mut self, ctx:&egui::Context, ui:&mut egui::Ui) {
        if self.current_window.is_some() {
            let window = &mut self.current_window.as_mut().unwrap().1;
            if window.should_appear() {
                window.render(ctx, ui);
            }
        }
    }

    pub fn set_window(&mut self, window:WindowDescriptor, shared_data:WindowSharedData) {
        match window.kind.clone() {
            EWindow::AddAccount => {
                self.current_window = Some((window.kind.clone(), Box::new(AddAccountWindow::new(shared_data, window.clone()))));
            },
            EWindow::CloneSettings(_info) => {
                self.current_window = Some((window.kind.clone(), Box::new(CloneControlsWindow::new(shared_data, window.clone()))));
            },
            EWindow::Settings => {
                self.current_window = Some((window.kind.clone(), Box::new(super::windows::settings::SettingsWindow::new(shared_data, window.clone()))));
            }
        }
    }
}

pub trait SubWindow {
    fn new(shared_data:WindowSharedData, window_descriptor:WindowDescriptor) -> Self where Self: Sized;
    fn create_window<'a>(&self, ui:&egui::Ui) -> egui::Window<'a> where Self:Sized;
    fn render(&mut self, ctx:&egui::Context, ui:&mut egui::Ui);
    fn close(&mut self);
    fn should_appear(&self) -> bool;
}


pub enum EventKind {
    Accounts(Vec<String>),
    AddToast(Toast),
    CurrentAccount(Option<String>)
}

pub type EventSender = Sender<EventKind>;
pub type EventReceiver = Receiver<EventKind>;
pub type EventManager = (EventSender, EventReceiver);

pub struct WindowSharedData {
    pub configuration: Arc<Mutex<crate::config::Configuration>>,
    pub accounts:Vec<String>,
    pub event_sender:EventSender
}