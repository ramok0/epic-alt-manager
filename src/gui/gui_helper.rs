use egui::{Button, FontId, Response, RichText};

use super::gui_constants::{
    BUTTON_MAX_SIZE, DELETE_COLOR, DELETE_COLOR_HOVER, PRIMARY_COLOR, PRIMARY_COLOR_HOVER,
    SECONDARY_COLOR, SECONDARY_COLOR_HOVER, TEXT_COLOR,
};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum EColor {
    Primary,
    #[allow(dead_code)]
    Secondary,
    Delete,
}

pub fn create_button<'a>(text: impl Into<String>) -> Button<'a> {
    Button::new(RichText::new(text).color(TEXT_COLOR))
}

pub fn add_button(ui: &mut egui::Ui, text: impl Into<String>, color: EColor) -> Response {
    match color {
        EColor::Primary => {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = PRIMARY_COLOR;
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = PRIMARY_COLOR_HOVER;
        }
        EColor::Secondary => {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = SECONDARY_COLOR;
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = SECONDARY_COLOR_HOVER;
        }
        EColor::Delete => {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = DELETE_COLOR;
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = DELETE_COLOR_HOVER;
        }
    }
    ui.add_sized(BUTTON_MAX_SIZE, create_button(text))
}

pub fn get_montserrat_font(font_size: f32) -> FontId {
    FontId::new(
        font_size,
        egui::FontFamily::Name("Montserrat".into()),
    )
}

pub fn rich_montserrat_text(text: impl Into<String>, font_size: f32) -> RichText {
    RichText::new(text).color(TEXT_COLOR).font(get_montserrat_font(font_size))
}

pub fn centerer(ui: &mut egui::Ui, id:impl std::hash::Hash, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let id = ui.id().with(id);
        let last_width: Option<f32> = ui.memory_mut(|mem| mem.data.get_temp(id));
        if let Some(last_width) = last_width {
            ui.add_space((ui.available_width() - last_width) / 2.0);
        }
        let res = ui
            .scope(|ui| {
                add_contents(ui);
            })
            .response;
        let width = res.rect.width();
        ui.memory_mut(|mem| mem.data.insert_temp(id, width));

        // Repaint if width changed
        match last_width {
            None => ui.ctx().request_repaint(),
            Some(last_width) if last_width != width => ui.ctx().request_repaint(),
            Some(_) => {}
        }
    });
}
