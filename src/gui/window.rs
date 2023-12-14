use std::sync::Arc;

use egui_toast::Toast;
use tokio::sync::{mpsc::Sender, Mutex};

use super::{gui_renderer::App, windows::add_account::AddAccountWindow};

#[derive(PartialEq, Clone, Hash, Eq, Debug)]
pub enum EWindow {
    AddAccount,
    Settings,
    About
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
            self.current_window.as_mut().unwrap().1.render(ctx, ui);
        }
    }

    pub fn set_window(&mut self, window:EWindow, shared_data:WindowSharedData) {
        match window {
            EWindow::AddAccount => {
                self.current_window = Some((window, Box::new(AddAccountWindow::new(shared_data))));
            },
            _ => {}
        }
    }
}

pub trait SubWindow {
    fn new(shared_data:WindowSharedData) -> Self where Self: Sized;
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

pub type EventManager = (tokio::sync::mpsc::Sender<EventKind>, tokio::sync::mpsc::Receiver<EventKind>);

pub struct WindowSharedData {
    pub configuration: Arc<Mutex<crate::config::Configuration>>,
    pub event_sender:tokio::sync::mpsc::Sender<EventKind>
}