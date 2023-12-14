use std::sync::Arc;

use egui_toast::Toast;
use tokio::sync::{mpsc::Sender, Mutex};

use super::gui_renderer::App;

#[derive(PartialEq, Clone, Hash, Eq, Debug)]
pub enum EWindow {
    AddAccount,
    Settings,
    About
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