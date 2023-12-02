use crate::config::Configuration;
use crate::egl::{ get_remember_me_data, FORTNITE_NEW_SWITCH_GAME_CLIENT };
use crate::epic::{ self, DeviceAuthorization, EpicError, EpicErrorKind };
use crate::gui::gui_constants::MODAL_COLOR;
use egui::epaint::Shadow;
use egui_toast::{ Toast, ToastKind, ToastOptions, Toasts };
use lazy_static::lazy_static;
use std::ffi::CString;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{ Receiver, Sender };
use tokio::sync::Mutex;
use windows::core::s;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
lazy_static! {
    // static ref THREAD_CREATION_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    // static ref THREAD_AUTHENTIFICATION_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    // static ref THREAD_ADD_ACCOUNT_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    // // flag qui va handle si le thread a ete crreer
    // static ref CREATE_CODE_THREAD_CREATED: AtomicBool = AtomicBool::new(false);
    // static ref AUTHENTIFICATION_THREAD_CREATED: AtomicBool = AtomicBool::new(false);
    // static ref ADD_ACCOUNT_THREAD_CREATED: AtomicBool = AtomicBool::new(false);

    static ref IS_ADDING_ACCOUNT: AtomicBool = AtomicBool::new(false);
}

use egui::{
    include_image,
    Align,
    Align2,
    Button,
    CursorIcon,
    FontId,
    Label,
    Layout,
    RichText,
    Sense,
    Pos2,
    Rounding,
    Color32,
    Stroke,
    CentralPanel,
    Margin,
    Image,
};
use windows::core::PCSTR;
use windows::Win32::UI::Shell::ShellExecuteA;

use super::gui_constants::{ DELETE_COLOR, PRIMARY_COLOR, TEXT_COLOR };
use super::gui_helper::{
    add_button,
    center_with_element,
    centerer,
    create_button,
    rich_montserrat_text,
    EColor,
    get_montserrat_font,
};

#[derive(Clone, Debug)]
pub struct AppDeviceAuthorization {
    pub(crate) device_code: DeviceAuthorization,
    pub(crate) device_code_created_at: Instant,
}

impl From<DeviceAuthorization> for AppDeviceAuthorization {
    fn from(value: DeviceAuthorization) -> Self {
        return Self {
            device_code: value,
            device_code_created_at: Instant::now(),
        };
    }
}

impl AppDeviceAuthorization {
    pub fn is_expired(&self) -> bool {
        Instant::now().duration_since(self.device_code_created_at).as_secs() >
            self.device_code.expires_in.try_into().unwrap()
    }
}
#[derive(Debug, Clone)]
pub struct SwapAccountInformation {
    swap_from: Option<String>,
    swap_to: String,
}

pub struct App {
    pub(crate) configuration: Arc<Mutex<Configuration>>,
    pub(crate) device_code_container: Arc<Mutex<Option<AppDeviceAuthorization>>>,
    pub(crate) device_code_clone: Option<DeviceAuthorization>,
    pub(crate) toasts: Toasts,
    accounts: Vec<String>,
    pub(crate) account_communication: (Sender<Vec<String>>, Receiver<Vec<String>>),
    device_code_communication: (
        Sender<Option<DeviceAuthorization>>,
        Receiver<Option<DeviceAuthorization>>,
    ),
    current_account: Option<String>,
    pub(crate) current_account_communication: (Sender<Option<String>>, Receiver<Option<String>>),
    pub(crate) toasts_communication: (Sender<Toast>, Receiver<Toast>),
    clone_configuration_information: Option<SwapAccountInformation>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let configuration = Configuration::new().expect("Failed to load configuration");
        let accounts = configuration.accounts.clone();

        if accounts.len() == 0 {
            IS_ADDING_ACCOUNT.store(true, Ordering::Relaxed);
        }

        let mut app = Self {
            configuration: Arc::new(Mutex::new(configuration)),
            device_code_clone: None,
            device_code_container: Arc::new(Mutex::new(None)),
            accounts: accounts
                .iter()
                .map(|x| x.display_name.clone())
                .collect(),
            account_communication: tokio::sync::mpsc::channel(std::mem::size_of::<Vec<String>>()),
            device_code_communication: tokio::sync::mpsc::channel(
                std::mem::size_of::<DeviceAuthorization>()
            ),
            current_account: None,
            current_account_communication: tokio::sync::mpsc::channel(
                std::mem::size_of::<String>()
            ),
            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-5.0, -5.0))
                .direction(egui::Direction::BottomUp),
            toasts_communication: tokio::sync::mpsc::channel(std::mem::size_of::<Toast>()),
            clone_configuration_information: None,
        };

        match get_remember_me_data() {
            Ok(account) => {
                app.current_account = Some(account.display_name.clone());
            }
            Err(_) => {
                app.toasts.add(Toast {
                    text: "Failed to get your EpicGames current account".into(),
                    kind: ToastKind::Error,
                    options: ToastOptions::default()
                        .duration_in_seconds(5.0)
                        .show_progress(true)
                        .show_icon(true),
                });
            }
        }

        app.auth_thread();

        app
    }

    fn auth_thread(&self) {
        let device_code_mtx = Arc::clone(&self.device_code_container);
        let configuration_mtx = Arc::clone(&self.configuration);
        let account_tx = self.account_communication.0.clone();
        let device_code_tx = self.device_code_communication.0.clone();
        let toast_communication = self.toasts_communication.0.clone();

        tokio::spawn(async move {
            loop {
                if IS_ADDING_ACCOUNT.load(Ordering::Relaxed) {
                    let mut current_device_code = device_code_mtx.lock().await;

                    if
                        current_device_code.is_none() ||
                        current_device_code.clone().unwrap().is_expired()
                    {
                        let new_device_code = Self::create_device_code().await;

                        if let Ok(device_code) = new_device_code {
                            let _ = device_code_tx.send(Some(device_code.clone())).await;
                            *current_device_code = Some(AppDeviceAuthorization::from(device_code));
                        }
                    } else {
                        let device_code_content = current_device_code.clone().unwrap();
                        tokio::time::sleep(
                            std::time::Duration::from_secs(
                                device_code_content.device_code.interval as u64
                            )
                        ).await;
                        let result = epic::token(
                            epic::Token::DeviceCode(&device_code_content.device_code.device_code),
                            FORTNITE_NEW_SWITCH_GAME_CLIENT
                        ).await;

                        match result {
                            Ok(account) => {
                                let mut configuration = configuration_mtx.lock().await;
                                let add_account_result = configuration.add_account(
                                    crate::config::AddAccountProvider::EpicAccount(&account)
                                ).await;

                                if add_account_result.is_ok() {
                                    let _ = toast_communication.send(Toast {
                                        text: RichText::new(
                                            format!(
                                                "Added {} successfully",
                                                account.display_name.clone().unwrap()
                                            )
                                        ).into(),
                                        kind: ToastKind::Info,
                                        options: ToastOptions::default()
                                            .duration_in_seconds(10.0)
                                            .show_progress(true)
                                            .show_icon(true),
                                    }).await;
                                }

                                if let Some(_display_name) = &account.display_name {
                                    let _ = account_tx.send(
                                        configuration.accounts
                                            .iter()
                                            .map(|x| x.display_name.clone())
                                            .collect()
                                    ).await;
                                }

                                let _ = configuration.flush();

                                //disable account add
                                *current_device_code = None;
                                let _ = device_code_tx.send(None).await;
                                IS_ADDING_ACCOUNT.store(false, Ordering::Relaxed);
                            }
                            Err(error) => {
                                eprintln!("Failed to login into account : {}", error);
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });
    }

    pub async fn create_device_code() -> Result<DeviceAuthorization, EpicError> {
        let client_token = epic::token(
            epic::Token::ClientCredentials,
            FORTNITE_NEW_SWITCH_GAME_CLIENT
        ).await?;

        return match client_token.get_device_authorization().await {
            Ok(result) => Ok(result),
            Err(_) =>
                Err(
                    EpicError::new(
                        EpicErrorKind::Authentification,
                        Some("Failed to get device authorization")
                    )
                ),
        };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.toasts.show(ctx);

        if let Ok(toast) = self.toasts_communication.1.try_recv() {
            self.toasts.add(toast);
        }

        if let Ok(new_accounts) = self.account_communication.1.try_recv() {
            self.accounts = new_accounts;
        }

        if let Ok(device_code) = self.device_code_communication.1.try_recv() {
            self.device_code_clone = device_code;
        }

        if let Ok(current_account) = self.current_account_communication.1.try_recv() {
            self.current_account = current_account;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!IS_ADDING_ACCOUNT.load(Ordering::Relaxed));

            ui.vertical_centered(|ui| {
                ui.add(
                    Label::new(
                        rich_montserrat_text("Smurf Manager", 22.0).strong().color(PRIMARY_COLOR)
                    )
                );

                ui.add_space(40.0);

                // ui.add(
                //     Label::new(
                //         rich_montserrat_text("Accounts", 15.)
                //     )
                // )
            });

            let mut sorted_accounts = self.accounts.clone();
            sorted_accounts.sort_by(|a, b| b.len().partial_cmp(&a.len()).unwrap());

            let longest_account = sorted_accounts.first();

            if let Some(username) = longest_account {
                const FONT_SIZE: f32 = 15.0;

                let mut y_offset = 10.0;

                ui.style_mut().spacing.item_spacing.y = 12.0;
                let font_id = get_montserrat_font(FONT_SIZE);

                for account in self.accounts.clone() {

                    let text_size = ui
                        .painter()
                        .layout_no_wrap(account.clone(), font_id.clone(), TEXT_COLOR)
                        .size();

                    let text_middle_screen = (ui.available_width() - text_size.x) / 2.0;
                    let screen_center: Pos2 = Pos2 { x: text_middle_screen, y: 50.0 };

                    let max_text = Pos2 {
                        x: screen_center.x + text_size.x + ui.style().spacing.item_spacing.x,
                        y: screen_center.y + text_size.y + y_offset,
                    };

                    let rect_text = egui::Rect {
                        min: Pos2 { x: screen_center.x, y: screen_center.y + y_offset },
                        max: max_text,
                    };

                    y_offset += text_size.y + ui.style().spacing.item_spacing.y;

                    let mut rect_configuration = rect_text.clone();

                    rect_configuration.min.x = screen_center.x - ui.style().spacing.item_spacing.x - 15.;
                    rect_configuration.max.x = screen_center.x - ui.style().spacing.item_spacing.x;
                    rect_configuration.min.y += 1.;
                    rect_configuration.max.y += 1.;                    
               
                        if ui.put(rect_configuration, 
                            egui::Image
                            ::new(include_image!("../../assets/icons/gear.svg"))
                            .sense(Sense::click())
                            .tint(PRIMARY_COLOR)
                            .max_width(15.0)
                        ).on_hover_cursor(CursorIcon::PointingHand).clicked(){
                            self.clone_configuration_information = Some(SwapAccountInformation {
                                 swap_from: None,
                                 swap_to: account.clone(),
                             });
                        }

              
                        if ui.put(
                            rect_text,
                            Label::new(rich_montserrat_text(account.clone(), FONT_SIZE).strong()).sense(Sense::click())
                        ).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            self.swap_account(account.clone());
                        }        //         if response {
                            //             self.swap_account(display_name.clone());
                            //         }

                        let mut rect_delete = rect_text.clone();

                        rect_delete.min.x = rect_text.max.x + ui.style().spacing.item_spacing.x;
                        rect_delete.max.x += ui.style().spacing.item_spacing.x + 15.;
                        rect_delete.min.y += 1.5;
                        rect_delete.max.y += 1.5;
     
                        if
                            ui
                                .put(
                                    rect_delete,
                                    egui::Image
                                        ::new(include_image!("../../assets/icons/trash.svg"))
                                        .sense(Sense::click())
                                        .tint(DELETE_COLOR)
                                        .max_width(15.0)
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                        {
                            self.remove_account(account.clone());
                        }
              

                }
            }

            // for account in self.accounts.clone() {
            //     let display_name = account.clone();

            //     let horizontal_align_id = ui.id().with("_center_with_elem");
            //     let text_height_id = ui.id().with("_center_text_and_svgs");

            //     let last_width: Option<f32> = ui.memory_mut(|mem|
            //         mem.data.get_temp(horizontal_align_id)
            //     );
            //     let last_text_height: Option<f32> = ui.memory_mut(|mem|
            //         mem.data.get_temp(text_height_id)
            //     );

            //     ui.horizontal(|ui| {
            //         if let Some(items_width) = last_width {
            //             ui.add_space((ui.available_width() - items_width) / 2.0);
            //         }

            //         if let Some(text_height) = last_text_height {
            //             let response = ui.put(
            //                 egui::Rect { min: Pos2 { x: 10., y: 50. }, max: Pos2 { x: 30., y: 80. } },
            //                 egui::Image
            //                     ::new(include_image!("../../assets/icons/gear.svg"))
            //                     .sense(Sense::click())
            //                     .tint(PRIMARY_COLOR)
            //                     .max_width(13.0)
            //             );

            //             let button_width = response.rect.width();

            //             if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            //                 self.clone_configuration_information = Some(SwapAccountInformation {
            //                     swap_from: None,
            //                     swap_to: display_name.clone(),
            //                 });
            //             }
            //         }

            //         let response = ui.add_sized(
            //             [135.0, 35.0],
            //             Label::new(rich_montserrat_text(account, 15.0).strong()).sense(
            //                 Sense::click()
            //             )
            //         );
            //         let text_width = response.rect.width();
            //         let text_height = response.rect.height();
   

            //         if
            //             ui
            //                 .add(
            //                     egui::Image
            //                         ::new(include_image!("../../assets/icons/trash.svg"))
            //                         .sense(Sense::click())
            //                         .tint(DELETE_COLOR)
            //                         .max_width(13.0)
            //                 )
            //                 .on_hover_cursor(CursorIcon::PointingHand)
            //                 .clicked()
            //         {
            //             self.remove_account(display_name.clone());
            //         }

            //         ui.memory_mut(|mem| mem.data.insert_temp(horizontal_align_id, text_width));
            //         ui.memory_mut(|mem| mem.data.insert_temp(text_height_id, text_height));
            //     });
            // }

            if self.clone_configuration_information.is_some() {
                let information = self.clone_configuration_information.clone().unwrap();

                egui::Window
                    ::new("Clone controls")
                    .resizable(false)
                    .collapsible(false)
                    .movable(false)
                    .title_bar(true)
                    .show(ctx, |ui| {
                        centerer(ui, |ui| {
                            ui.label("I want to clone controls from ");

                            egui::ComboBox
                                ::from_id_source("account selector")
                                .selected_text(
                                    information.swap_from
                                        .clone()
                                        .unwrap_or(String::from("Select account"))
                                )
                                .show_ui(ui, |ui| {
                                    let mut selectable_string = |
                                        ui: &mut egui::Ui,
                                        element: String
                                    | {
                                        let info = information.clone();
                                        let currently_selected =
                                            info.swap_from.is_some() &&
                                            info.swap_from.unwrap() == element.clone();
                                        let mut response = ui.selectable_label(
                                            currently_selected,
                                            RichText::new(element.clone())
                                        );
                                        if response.clicked() && !currently_selected {
                                            self.clone_configuration_information = Some(
                                                SwapAccountInformation {
                                                    swap_to: info.swap_to,
                                                    swap_from: Some(element),
                                                }
                                            );
                                            response.mark_changed();
                                        }
                                        response
                                    };

                                    for account in self.accounts.clone() {
                                        if account != information.swap_to {
                                            selectable_string(ui, account);
                                        }
                                    }
                                });

                            ui.label(format!("to {}", information.swap_to));
                        });
                    });
            }
            if self.device_code_clone.is_some() {
                let text =
                    "Use this code to link your account to this application through epicgames.com/activate".to_string();
                let font_id = FontId::new(14.0, egui::FontFamily::Name("Roboto".into()));

                let text_size = ui
                    .painter()
                    .layout_no_wrap(text.clone(), font_id.clone(), TEXT_COLOR)
                    .size();

                egui::Window
                    ::new("Add a new account")
                    .resizable(false)
                    .collapsible(false)
                    .movable(false)
                    .min_width(text_size.x)
                    .title_bar(false)
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.style_mut().spacing.item_spacing.y += 10.0;
                        ui.style_mut().spacing.window_margin.bottom += 10.0;
                        ui.style_mut().spacing.window_margin.top += 10.0;

                        let data = self.device_code_clone.clone().unwrap();

                        ui.vertical_centered(|ui| {
                            if
                                ui
                                    .add(
                                        Label::new(
                                            rich_montserrat_text(
                                                data.user_code.clone(),
                                                18.0
                                            ).heading()
                                        ).sense(Sense::click())
                                    )
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                            {
                                ui.output_mut(|x| {
                                    x.copied_text = data.user_code.clone();
                                });
                            }
                        });

                        ui.add(Label::new(RichText::new(text).color(TEXT_COLOR).font(font_id)));

                        centerer(ui, |ui| {
                            if add_button(ui, "Link my account", EColor::Primary).clicked() {
                                let url = data.verification_uri;
                                let url_cstring = CString::new(url).expect(
                                    "Failed to convert url from String to CString"
                                );
                                unsafe {
                                    ShellExecuteA(
                                        windows::Win32::Foundation::HWND(0),
                                        s!("open"),
                                        PCSTR::from_raw(url_cstring.as_bytes_with_nul().as_ptr()),
                                        s!(""),
                                        s!(""),
                                        SW_SHOW
                                    );
                                }
                            }

                            if add_button(ui, "Cancel", EColor::Delete).clicked() {
                                IS_ADDING_ACCOUNT.store(false, Ordering::Relaxed);
                                self.device_code_clone = None;
                            }
                        });
                    });
            }

            //action menu at the bottom of the app
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.style_mut().spacing.item_spacing.y += 3.0;

                if
                    self.current_account.is_some() &&
                    !self.accounts.contains(&self.current_account.clone().unwrap())
                {
                    if
                        ui
                            .add_sized(
                                [280.0, 36.0],
                                create_button("Link my current EpicGamesLauncher account")
                            )
                            .clicked()
                    {
                        self.link_egl_account();
                    }
                }

                if add_button(ui, "Kill EpicGamesLauncher", EColor::Primary).clicked() {
                    let result = Self::kill_epic_games_launcher().map(|_| {
                        Toast {
                            kind: ToastKind::Success,
                            text: egui::WidgetText::RichText(
                                RichText::new("Killed EpicGamesLauncher successfully")
                            ),
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_icon(true)
                                .show_progress(true),
                        }
                    });

                    self.toasts.add(match result {
                        Ok(toast) => toast,
                        Err(epic_error) => epic_error.to_toast(),
                    });
                }

                if !IS_ADDING_ACCOUNT.load(Ordering::Relaxed) {
                    if add_button(ui, "Link another account", EColor::Primary).clicked() {
                        IS_ADDING_ACCOUNT.store(true, Ordering::Relaxed);
                    }
                }
            });
        });
    }
}
