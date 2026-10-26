//! The work bar under the top bar: Importa, Sviluppa, Esporta.
//!
//! Importa opens the import picker, Sviluppa opens the photo in the Edit panel with the filmstrip
//! below it, Esporta opens the export dialog.

use egui::{Align2, Rect, Sense, Stroke, StrokeKind, vec2};
use serde_json::json;

use crate::LightcraftApp;
use crate::state::{RightPanel, ViewMode};
use crate::theme::Tokens;
use crate::widgets::register;

/// Height of the bar.
const BAR_H: f32 = 40.0;
const BUTTON_W: f32 = 104.0;
const BUTTON_H: f32 = 28.0;

/// What a button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Import,
    Develop,
    Export,
}

impl Action {
    pub const ALL: [Action; 3] = [Action::Import, Action::Develop, Action::Export];

    pub fn label(self) -> &'static str {
        match self {
            Action::Import => "Importa",
            Action::Develop => "Sviluppa",
            Action::Export => "Esporta",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Action::Import => "workbar:import",
            Action::Develop => "workbar:develop",
            Action::Export => "workbar:export",
        }
    }
}

/// Run a button's action. Sviluppa picks the first photo when none is open, then shows the photo
/// with the Edit panel and the filmstrip.
pub fn run(app: &mut LightcraftApp, action: Action) {
    match action {
        Action::Import => {
            let _ = app.run("file.addFolder", json!({}));
        }
        Action::Develop => {
            if app.session.active().is_none()
                && let Some(first) = app.session.visible_cloned().first().copied()
            {
                let _ = app.run("library.select", json!({"ids": [first.0]}));
            }
            if app.session.active().is_none() {
                return;
            }
            let _ = app.run("view.detail", json!({}));
            if app.ui.right == RightPanel::None {
                let _ = app.run("panel.edit", json!({}));
            }
            app.ui.filmstrip = true;
        }
        Action::Export => {
            let _ = app.run("dialog.export", json!({}));
        }
    }
}

fn enabled(app: &mut LightcraftApp, action: Action) -> bool {
    match action {
        Action::Import => true,
        Action::Develop | Action::Export => !app.session.visible().is_empty(),
    }
}

pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("work_bar")
        .exact_size(BAR_H)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(t.chrome)
                .inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 0 })
                .stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for action in Action::ALL {
                    let on = action == Action::Develop && app.ui.view == ViewMode::Detail;
                    let ok = enabled(app, action);
                    let (rect, resp) = ui.allocate_exact_size(vec2(BUTTON_W, BUTTON_H), if ok { Sense::click() } else { Sense::hover() });
                    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ok, on, action.label()));
                    register(ui.ctx(), action.id(), rect);
                    paint_button(ui, rect, action.label(), on, ok, resp.hovered(), &t);
                    if ok && resp.clicked() {
                        run(app, action);
                    }
                }
            });
        });
}

fn paint_button(ui: &egui::Ui, rect: Rect, label: &str, on: bool, enabled: bool, hovered: bool, t: &Tokens) {
    let fill = if on {
        t.accent
    } else if hovered && enabled {
        t.hover
    } else {
        t.field
    };
    let text = if on {
        egui::Color32::WHITE
    } else if enabled {
        t.text
    } else {
        t.text_dim
    };
    let p = ui.painter();
    p.rect(rect, 6.0, fill, Stroke::new(1.0, if on { t.accent } else { t.field_border }), StrokeKind::Inside);
    p.text(rect.center(), Align2::CENTER_CENTER, label, t.semibold(13.5), text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_buttons_are_import_develop_export_in_that_order() {
        let labels: Vec<_> = Action::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(labels, ["Importa", "Sviluppa", "Esporta"]);
    }
}
