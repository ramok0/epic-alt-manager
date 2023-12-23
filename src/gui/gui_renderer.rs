use crate::config::Configuration;
use crate::egl::epic_get_remember_me_data;
use crate::epic::DeviceAuthorization;
use egui_toast::{ Toast, ToastKind, ToastOptions, Toasts };

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use egui::{
    include_image,
    Align,
    Align2,
    CursorIcon,
    Label,
    Layout,
    RichText,
    Sense,
    Pos2,
    Image,
    Vec2,
};

use super::gui_constants::{ DELETE_COLOR, PRIMARY_COLOR, TEXT_COLOR };
use super::gui_helper::{
    add_button,
    create_button,
    rich_montserrat_text,
    EColor,
    get_montserrat_font,
};
use super::window::{EventManager, EventKind, EWindow, WindowManager};
use super::windows::clone_configuration::CloneControlsData;
use super::windows::settings::RuntimeSettings;

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

pub struct App {
    pub configuration: Arc<Mutex<Configuration>>,
    pub toasts: Toasts,
    pub(crate) accounts: Vec<String>,
    pub(crate) current_account: Option<String>,
    pub runtime_settings:Arc<std::sync::Mutex<RuntimeSettings>>,
    pub(crate) window_manager: WindowManager,
    pub event_manager: EventManager,
}

impl Default for App {
    fn default() -> Self {
        let configuration = Configuration::new().expect("Failed to load configuration");
        let accounts = configuration.accounts.clone();

        Self {
            configuration: Arc::new(Mutex::new(configuration)),
            accounts: accounts
                .iter()
                .map(|x| x.display_name.clone())
                .collect(),
            current_account: None,
            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-5.0, -5.0))
                .direction(egui::Direction::BottomUp),
            runtime_settings: Arc::new(std::sync::Mutex::new(RuntimeSettings { advanced_mode: false })),
            event_manager: tokio::sync::mpsc::channel(std::mem::size_of::<EventKind>()),
            window_manager: WindowManager::new()
        }
    }
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App::default();

        if app.accounts.len() == 0 {
            app.set_window(EWindow::AddAccount);
        }

        match epic_get_remember_me_data() {
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
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.toasts.show(ctx);
        self.handle_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            //disable window controls if a subwindow is opened and should render
            let should_disable = self.window_manager.current_window.is_some() && self.window_manager.current_window.as_ref().unwrap().1.should_appear();
            ui.set_enabled(!should_disable);

            ui.vertical_centered(|ui| {
                ui.add(
                    Label::new(
                        rich_montserrat_text("Smurf Manager", 22.0).strong().color(PRIMARY_COLOR)
                    )
                );
            });

            let mut sorted_accounts = self.accounts.clone();
            sorted_accounts.sort_by(|a, b| b.len().partial_cmp(&a.len()).unwrap());

            let longest_account = sorted_accounts.first();

            if let Some(username) = longest_account {
                const FONT_SIZE: f32 = 15.0;

                let base_y_offset = ui.available_height() / 5.0;
                let mut y_offset = 0.0;

                ui.style_mut().spacing.item_spacing.y = 12.0;
                let font_id = get_montserrat_font(FONT_SIZE);

                let text_size = ui
                    .painter()
                    .layout_no_wrap(username.clone(), font_id.clone(), TEXT_COLOR)
                    .size();

                let text_middle_screen = (ui.available_width() - text_size.x) / 2.0;

                for account in self.accounts.clone() {
                    let screen_center: Pos2 = Pos2 { x: text_middle_screen, y: base_y_offset };

                    let max_text = Pos2 {
                        x: screen_center.x + text_size.x + ui.style().spacing.item_spacing.x,
                        y: screen_center.y + text_size.y + y_offset,
                    };

                    let rect_text = egui::Rect {
                        min: Pos2 { x: screen_center.x, y: screen_center.y + y_offset },
                        max: max_text,
                    };

                    y_offset += text_size.y + ui.style().spacing.item_spacing.y;

                    let mut rect_controls = rect_text.clone();

                    rect_controls.min.x = screen_center.x - ui.style().spacing.item_spacing.x - 15.0;
                    rect_controls.max.x = screen_center.x - ui.style().spacing.item_spacing.x;
                    rect_controls.min.y += 1.0;
                    rect_controls.max.y += 1.0;

                    if
                        ui
                            .put(
                                rect_controls,
                                egui::Image
                                    ::new(include_image!("../../assets/icons/clipboard.svg"))
                                    .sense(Sense::click())
                                    .tint(PRIMARY_COLOR)
                                    .max_width(15.0)
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                    {
                        self.set_window(EWindow::CloneSettings(CloneControlsData {
                            clone_from: None,
                            clone_to: account.clone(),
                        }));
                    }

                    if
                        ui
                            .put(
                                rect_text,
                                Label::new(
                                    rich_montserrat_text(account.clone(), FONT_SIZE).strong()
                                ).sense(Sense::click())
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                    {
                        self.swap_account(account.clone());
                    }

                    let mut rect_delete = rect_text.clone();

                    rect_delete.min.x = rect_text.max.x + ui.style().spacing.item_spacing.x;
                    rect_delete.max.x += ui.style().spacing.item_spacing.x + 15.0;
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

            self.window_manager.render(ctx, ui);

            //action menu at the bottom of the app
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.style_mut().spacing.item_spacing.y = 5.0;

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

                if add_button(ui, "Kill EGL",  EColor::Primary).clicked() {
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

                    //afficher le bouton pour ajouter un compte

                    if let Some(window_pos) = ui.input(|i| { i.viewport().inner_rect }) {
                        let window_size = window_pos.max - window_pos.min;
                        let svg_area = ((window_size.x * window_size.y) / 768.).sqrt().clamp(20., 30.);
    
                        let plus_max = Vec2 { 
                            x: window_size.x * 0.99, 
                            y: window_size.y * 0.991
                        };
    
                        let plus_min = plus_max - Vec2 { x: svg_area, y: svg_area };
                        let vec_to_pos = |x: Vec2| -> Pos2 { Pos2 { x: x.x, y: x.y } };
    
            
                        let response = ui.put(
                            egui::Rect { min: vec_to_pos(plus_min), max: vec_to_pos(plus_max) },
                            Image::new(include_image!("../../assets/icons/plus.svg"))
                            .sense(Sense::click())
                            .tint(PRIMARY_COLOR)
                            .max_width(svg_area)
                        );
    
                        if response.clicked() {
                            self.set_window(EWindow::AddAccount);
                        }
    
                        let configuration_max = Vec2 {
                            x: plus_min.x * 0.98,
                            y: plus_max.y-1.
                        };
    
    
                         let configuration_min = configuration_max - Vec2 { x: svg_area, y: svg_area };
                         let configuration_rect = egui::Rect {min: vec_to_pos(configuration_min), max: vec_to_pos(configuration_max)};
    
    
                        if ui.put(
                            configuration_rect,
                            Image::new(include_image!("../../assets/icons/gear.svg"))
                            .sense(Sense::click())
                            .tint(PRIMARY_COLOR)
                            .max_width(svg_area))
                            .clicked() {
                                self.set_window(EWindow::Settings);
                            }
                    }                
            });
        });
    }
}
