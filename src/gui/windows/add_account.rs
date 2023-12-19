use std::sync::Arc;
use tokio::sync::{mpsc::{Sender, Receiver}, Mutex};

use egui::{FontId, Align2, CursorIcon, Sense, Label, RichText, OpenUrl};
use egui_toast::{Toast, ToastOptions};

use crate::{gui::{window::{SubWindow, WindowSharedData, WindowDescriptor}, gui_constants::TEXT_COLOR, gui_renderer::AppDeviceAuthorization, gui_helper::{rich_montserrat_text, centerer, add_button, EColor}}, epic::{TokenType, DeviceAuthorization, self, EpicError}, egl::FORTNITE_NEW_SWITCH_GAME_CLIENT};

#[derive(Debug, Default, Clone)]
pub struct CredentialsBuffer {
    pub data:String,
    pub client_id:String,
    pub client_secret:String
}

#[derive(Debug, Default, Clone)]
pub struct DeviceAuthBuffer {
    pub account_id:String,
    pub device_id:String,
    pub secret:String,
    pub client_id:String,
    pub client_secret:String
}

pub struct AddAccountWindow {
    pub refresh_token_buffer:CredentialsBuffer,
    pub authorization_code_buffer:CredentialsBuffer,
    pub exchange_code_buffer:CredentialsBuffer,
    pub device_auth_buffer:DeviceAuthBuffer,
    font:FontId,
    _add_type:TokenType,
    device_code_clone:Option<DeviceAuthorization>,
    device_code_container: Arc<Mutex<Option<AppDeviceAuthorization>>>,
    device_code_communication: (Sender<AppDeviceAuthorization>, Receiver<AppDeviceAuthorization>),
    shared_data:WindowSharedData,
    thread_state:Arc<Mutex<bool>>, //thread unique par window
    should_close: bool,
    close_window_communication: (Sender<bool>, Receiver<bool>)
}

const MAIN_TEXT:&'static str = "Use this code to link your account to this application through epicgames.com/activate";

macro_rules! manage_error {
    ($result:ident, $event_sender:ident) => {
        match $result {
            Ok(data) => data,
            Err(error) => {
                let _ = $event_sender.send(crate::gui::window::EventKind::AddToast(error.to_toast()));
                tokio::time::sleep(std::time::Duration::from_millis(1250)).await;
                continue;
            }
        }
    };
}

impl AddAccountWindow {
    pub fn device_code_manager(&self) -> () {
        let device_code_container = Arc::clone(&self.device_code_container);
        let device_code_communication = self.device_code_communication.0.clone();
        let close_window_communication = self.close_window_communication.0.clone();

        let thread_state = self.thread_state.clone();
        let event_sender = self.shared_data.event_sender.clone();
        let configuration_mtx = Arc::clone(&self.shared_data.configuration);
        tokio::spawn(async move {
            loop {
                if *thread_state.lock().await == false {
                    break;
                }

                let mut current_device_code = device_code_container.lock().await;

                //if code is none or expired, get a new one
                if current_device_code.is_none() || current_device_code.as_ref().unwrap().is_expired() {
                    let client_token_result = epic::token(
                        epic::Token::ClientCredentials,
                        FORTNITE_NEW_SWITCH_GAME_CLIENT
                    ).await;


                    let client_token = manage_error!(client_token_result, event_sender);
                    let device_authorization_result = client_token.get_device_authorization().await;
                    let device_authorization = AppDeviceAuthorization::from( manage_error!(device_authorization_result, event_sender));
                    
                    let _ = device_code_communication.send(device_authorization.clone()).await;
                    *current_device_code = Some(device_authorization);

                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    continue;
                }

                let _ = device_code_communication.send(current_device_code.as_ref().unwrap().clone()).await;
                let app_device_code = current_device_code.as_ref().unwrap().clone();

                tokio::time::sleep(std::time::Duration::from_secs(app_device_code.device_code.interval as u64)).await;

                let login_result = epic::token(epic::Token::DeviceCode(&app_device_code.device_code.device_code), FORTNITE_NEW_SWITCH_GAME_CLIENT).await;            
                let account = manage_error!(login_result, event_sender);

                //add account into configuration and show a toast to the user

                let mut configuration = configuration_mtx.lock().await;
                let add_account_result = configuration.add_account(crate::config::AddAccountProvider::EpicAccount(&account)).await.map_err(|_| EpicError::new(epic::EpicErrorKind::Other, Some("Failed to add account to configuration")));

                let _ = manage_error!(add_account_result, event_sender);

                let _ = event_sender.send(crate::gui::window::EventKind::AddToast(
                    Toast {
                        kind: egui_toast::ToastKind::Success,
                        text: RichText::new(format!("Linked {} succesfully.", account.display_name.as_ref().unwrap())).into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(10.0)
                            .show_progress(true)
                            .show_icon(true),
                    }
                )).await;

                let _ = event_sender.send(crate::gui::window::EventKind::Accounts(configuration.accounts.iter().map(|x| x.display_name.clone()).collect())).await;
                let _ = configuration.flush();

                let _ = close_window_communication.send(true).await;

                break;
                //disable account adding mode
           //     WANTS_TO_SHOW_ACCOUNT_WINDOW.store(false, Ordering::Relaxed);
            }
        });
    }
}

impl SubWindow for AddAccountWindow {
    fn new(shared_data:WindowSharedData, _description:WindowDescriptor) -> Self where Self: Sized {
        //WANTS_TO_SHOW_ACCOUNT_WINDOW.store(true, Ordering::Relaxed);
        let font = FontId::new(14., egui::FontFamily::Name("Roboto".into()));

        let window = Self {
            refresh_token_buffer:CredentialsBuffer::default(),
            authorization_code_buffer:CredentialsBuffer::default(),
            exchange_code_buffer:CredentialsBuffer::default(),
            device_auth_buffer:DeviceAuthBuffer::default(),
            font,
            _add_type: TokenType::DeviceCode,

            device_code_clone: None,
            device_code_container: Arc::new(Mutex::new(None)),
            device_code_communication: tokio::sync::mpsc::channel(std::mem::size_of::<AppDeviceAuthorization>()),
            shared_data,
            thread_state: Arc::new(Mutex::new(true)),
            should_close: false,
            close_window_communication: tokio::sync::mpsc::channel(1)
        };

        window.device_code_manager();

        window
    }

    fn render(&mut self, ctx:&egui::Context, ui:&mut egui::Ui) {
        if let Some(device_code) = self.device_code_communication.1.try_recv().ok() {
            self.device_code_clone = Some(device_code.device_code);
        }

        if let Some(close_window) = self.close_window_communication.1.try_recv().ok() {
            self.should_close = close_window;
        } 

        self.create_window(ui).show(ctx, |ui| {
            // if self.add_type == TokenType::DeviceCode {

            // }

            if let Some(device_code) = self.device_code_clone.clone() {
                ui.vertical_centered(|ui| {
                    if
                        ui
                            .add(
                                Label::new(
                                    rich_montserrat_text(
                                        device_code.user_code.clone(),
                                        18.0
                                    ).heading()
                                ).sense(Sense::click())
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                    {
                        ui.output_mut(|x| {
                            x.copied_text = device_code.user_code.clone();
                        });
                    }
                });

                ui.add(Label::new(RichText::new(MAIN_TEXT).color(TEXT_COLOR).font(self.font.clone())));
    
                centerer(ui, "_link_account", |ui| {
                    if add_button(ui, "Link my account", EColor::Primary).clicked() {
                        let url = device_code.verification_uri;
                        ctx.open_url(OpenUrl { url, new_tab: true });
                    }

                    if add_button(ui, "Close", EColor::Delete).clicked() {
                 //       WANTS_TO_SHOW_ACCOUNT_WINDOW.store(false, Ordering::Relaxed);
                        let state = self.thread_state.clone();
                        tokio::spawn(async move {
                            *state.lock().await = false;
                        });

                        self.should_close = true;
                    }
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.label(rich_montserrat_text("Please wait a second...", 15.0));
                });
            }


        });
    }

    fn close(&mut self) {

    }

    fn should_appear(&self) -> bool {
        !self.should_close
    }

    fn create_window<'a>(&self, ui:&egui::Ui) -> egui::Window<'a> where Self:Sized {
        let text_size = ui
        .painter()
        .layout_no_wrap(MAIN_TEXT.to_owned(), self.font.clone(), TEXT_COLOR)
        .size();

        egui::Window::new("Add a new account")
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .min_width(text_size.x)
            .title_bar(false)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
    }
}

impl Drop for AddAccountWindow {
    fn drop(&mut self) {
        println!("window dropped");
    }
}