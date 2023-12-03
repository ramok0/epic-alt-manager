use std::time::Duration;

use crate::egl::get_decryption_keys;
use egui::{Color32, Rounding, Stroke, Style, Visuals};
use gui::gui_constants::{MODAL_COLOR, TEXT_COLOR};

mod config;
mod decrypt;
mod egl;
mod epic;
mod gui;
mod process;

use eframe::{egui, NativeOptions};
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().expect("Unable to create Runtime");

    let _enter = rt.enter();

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    Runtime::new().unwrap().block_on(async {
        get_decryption_keys().await;
    });

    // Run the GUI in the main thread.
    let options = NativeOptions::default();

    let _ = eframe::run_native(
        "Alt Manager",
        options,
        Box::new(|cc| {
            let mut style = Style {
                visuals: Visuals {
                    panel_fill: Color32::from_rgb(0x0f, 0x0f, 0x0f),
                    window_rounding: Rounding::same(5.),
                    window_fill: MODAL_COLOR,
                    ..Visuals::default()
                },
                ..Style::default()
            };

            style.visuals.widgets.noninteractive.fg_stroke =
                Stroke::new(1.0, Color32::from_rgb(0xf2, 0xf0, 0xff));
            style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x03, 0x68, 0xff);
            style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x58, 0x99, 0xff);
            style.visuals.widgets.inactive.rounding = Rounding::same(7.);
            style.visuals.widgets.hovered.rounding = Rounding::same(7.);

            style.visuals.override_text_color = Some(TEXT_COLOR);

            style.spacing.window_margin.bottom = 10.;
            style.spacing.window_margin.top = 10.;
            style.spacing.window_margin.right = 10.;
            style.spacing.window_margin.left = 10.;

            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Monserrat".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Montserrat-Bold.ttf")),
            );

            fonts
                .families
                .entry(egui::FontFamily::Name("Montserrat".into()))
                .or_default()
                .insert(0, "Monserrat".to_owned());

            fonts.font_data.insert(
                "Roboto".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Roboto-Regular.ttf")),
            );

            fonts
                .families
                .entry(egui::FontFamily::Name("Roboto".into()))
                .or_default()
                .insert(0, "Roboto".to_owned());

            cc.egui_ctx.set_fonts(fonts);
            cc.egui_ctx.set_style(style);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(crate::gui::gui_renderer::App::new(cc))
        }),
    );
}
