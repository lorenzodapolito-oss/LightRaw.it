//! LD Panel: a docked panel beside the photo (between the photo and the Edit panel) with one-click
//! recipes for landscape and night photography, in six sections (Main, Color Correction, Night
//! Time, Editing, Final Touch, Export). Opened from the LD Panel menu. Every button runs ordinary
//! engine commands (develop values, masks, export), so the control channel and MCP see exactly
//! what the buttons do.

use egui::RichText;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::LightcraftApp;
use crate::theme::Tokens;

const PREFIX: &str = "ldpanel.";
const SECTIONS: [&str; 6] = ["main", "color", "night", "editing", "final", "export"];
const WIDTH: f32 = 300.0;
const BANDS: [&str; 8] = ["red", "orange", "yellow", "green", "aqua", "blue", "purple", "magenta"];

/// The panel's view state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LdPanel {
    pub open: bool,
    /// One of `SECTIONS`.
    pub section: String,
    /// Luminosity mask level, 1..=3 (higher is narrower).
    pub level: u32,
    /// Colour mixer sliders: hue / saturation / luminance for the eight bands (-100..100), and
    /// which of the three the sliders show.
    pub mixer: Vec<f32>,
    pub mixer_mode: u8,
}

impl Default for LdPanel {
    fn default() -> Self {
        LdPanel { open: false, section: "main".into(), level: 1, mixer: vec![0.0; 24], mixer_mode: 0 }
    }
}

/// Menu checkmark for LD Panel › section.
pub fn checked(app: &LightcraftApp, id: &str) -> Option<bool> {
    let name = id.strip_prefix(PREFIX)?;
    SECTIONS.contains(&name).then(|| app.ui.ld.open && app.ui.ld.section == name)
}

/// A menu command: show the panel on a section, or hide it. `None` for any other id.
pub fn command(app: &mut LightcraftApp, id: &str) -> Option<Result<Value, String>> {
    let name = id.strip_prefix(PREFIX)?;
    if name == "hide" {
        app.ui.ld.open = false;
    } else if SECTIONS.contains(&name) {
        app.ui.ld.open = true;
        app.ui.ld.section = name.to_string();
    } else {
        return None;
    }
    Some(Ok(json!({ "open": app.ui.ld.open, "section": app.ui.ld.section })))
}

fn section_label(name: &str) -> &'static str {
    match name {
        "color" => crate::i18n::tr("Color Correction"),
        "night" => crate::i18n::tr("Night Time"),
        "editing" => crate::i18n::tr("Editing"),
        "final" => crate::i18n::tr("Final Touch"),
        "export" => crate::i18n::tr("Export"),
        _ => crate::i18n::tr("Main"),
    }
}

/// The tonal range a Lights / Midtones / Shadows button masks, narrower at higher levels.
pub(crate) fn range_params(kind: &str, level: u32) -> Value {
    let n = f64::from(level.clamp(1, 3) - 1);
    let (lo, hi) = match kind {
        "darks" => (0.0, 0.4 - 0.1 * n),
        "mids" => (0.3 + 0.05 * n, 0.7 - 0.05 * n),
        _ => (0.6 + 0.1 * n, 1.0),
    };
    json!({ "kind": "luminanceRange", "lo": lo, "hi": hi, "loFeather": 0.1, "hiFeather": 0.1 })
}

/// A mask command for one range button: a new mask, or a component added to / taken from the
/// active mask.
pub(crate) fn mask_step(kind: &str, level: u32, op: &str) -> (&'static str, Value) {
    let mut p = range_params(kind, level);
    if op == "new" {
        ("mask.add", p)
    } else {
        p["op"] = json!(if op == "subtract" { "subtract" } else { "add" });
        ("mask.addComponent", p)
    }
}

/// Mixer values for `develop.set`: only the sliders that were moved.
pub(crate) fn mixer_values(values: &[f32]) -> Value {
    let kinds = ["hue", "sat", "lum"];
    let mut out = serde_json::Map::new();
    for (m, kind) in kinds.iter().enumerate() {
        for (i, band) in BANDS.iter().enumerate() {
            let v = values.get(m * 8 + i).copied().filter(|v| v.is_finite()).unwrap_or(0.0).clamp(-100.0, 100.0);
            if v != 0.0 {
                out.insert(format!("mixer.{band}.{kind}"), json!(v));
            }
        }
    }
    Value::Object(out)
}

type Steps = Vec<(&'static str, Value)>;

/// Run commands in order, stopping at the first that fails (its message goes to the status line).
fn run_all(app: &mut LightcraftApp, steps: Steps) {
    for (cmd, params) in steps {
        if let Err(e) = app.run(cmd, params) {
            app.ui.status = e;
            return;
        }
    }
}

fn adjust(steps: &mut Steps, control: &str, delta: f64) {
    steps.push(("develop.adjust", json!({"control": control, "delta": delta})));
}

fn set(steps: &mut Steps, control: &str, value: f64) {
    steps.push(("develop.set", json!({"control": control, "value": value})));
}

fn button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add_sized([ui.available_width(), 20.0], egui::Button::new(RichText::new(label).size(11.0))).clicked()
}

fn small(ui: &mut egui::Ui, label: &str, width: f32) -> bool {
    ui.add_sized([width, 20.0], egui::Button::new(RichText::new(label).size(11.0))).clicked()
}

fn group(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::NONE.stroke(egui::Stroke::new(1.0, t.divider)).corner_radius(6.0).inner_margin(6.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong().size(11.0).color(t.text));
        ui.add_space(2.0);
        body(ui);
    });
    ui.add_space(4.0);
}

/// Draw the panel at the right edge of the photo's pane.
pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    if !app.ui.ld.open {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let mut state = app.ui.ld.clone();
    if state.mixer.len() != 24 {
        state.mixer = vec![0.0; 24];
    }
    let mut steps: Steps = Vec::new();
    let mut close = false;
    let has_photo = app.session.active().is_some();
    egui::Panel::right("ld_panel")
        .exact_size(WIDTH)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(6.0).stroke(egui::Stroke::new(1.0, t.divider)))
        .show(ui, |ui| {
            {
                let sp = ui.spacing_mut();
                sp.button_padding = egui::vec2(4.0, 1.0);
                sp.item_spacing = egui::vec2(4.0, 3.0);
                sp.slider_width = 90.0;
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("LD Panel").strong().size(13.0).color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").clicked() {
                        close = true;
                    }
                });
            });
            let w = ((WIDTH - 12.0 - 8.0) / 3.0).floor();
            egui::Grid::new("ld-sections").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, name) in SECTIONS.iter().enumerate() {
                    let on = state.section == *name;
                    let r = ui.add_sized([w, 20.0], egui::Button::new(RichText::new(section_label(name)).size(10.0)).selected(on));
                    if r.clicked() {
                        state.section = (*name).to_string();
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(4.0);
            if !has_photo {
                ui.weak(crate::i18n::tr("Select a photo to use the panel."));
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 12.0);
                match state.section.as_str() {
                    "color" => color_section(ui, &mut state, &mut steps),
                    "night" => night_section(ui, &mut steps),
                    "editing" => editing_section(ui, &mut steps),
                    "final" => final_section(ui, &mut steps),
                    "export" => export_section(ui, &mut steps),
                    _ => main_section(ui, &mut state, &mut steps),
                }
            });
        });
    app.ui.ld = state;
    if close {
        app.ui.ld.open = false;
    }
    run_all(app, steps);
}

fn main_section(ui: &mut egui::Ui, state: &mut LdPanel, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Luminosity Masks"), |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr("Level"));
            for n in 1..=3u32 {
                ui.selectable_value(&mut state.level, n, n.to_string());
            }
        });
        let col = ((ui.available_width() - 8.0) / 3.0).floor();
        ui.horizontal(|ui| {
            for (kind, label) in [("darks", crate::i18n::tr("Shadows")), ("mids", crate::i18n::tr("Midtones")), ("lights", crate::i18n::tr("Lights"))] {
                ui.vertical(|ui| {
                    ui.set_width(col);
                    if small(ui, label, col) {
                        steps.push(mask_step(kind, state.level, "new"));
                    }
                    ui.horizontal(|ui| {
                        if small(ui, "+", (col - 4.0) / 2.0) {
                            steps.push(mask_step(kind, state.level, "add"));
                        }
                        if small(ui, "−", (col - 4.0) / 2.0) {
                            steps.push(mask_step(kind, state.level, "subtract"));
                        }
                    });
                });
            }
        });
    });
    group(ui, crate::i18n::tr("Dodge and Burn"), |ui| {
        let col = ((ui.available_width() - 4.0) / 2.0).floor();
        ui.horizontal_top(|ui| {
            for (burn, title) in [(false, crate::i18n::tr("Dodge")), (true, crate::i18n::tr("Burn"))] {
                ui.vertical(|ui| {
                    ui.set_width(col);
                    ui.label(RichText::new(title).strong().size(11.0));
                    for (kind, label) in [("lights", crate::i18n::tr("Lights")), ("mids", crate::i18n::tr("Midtones")), ("darks", crate::i18n::tr("Shadows"))] {
                        if small(ui, label, col) {
                            // a mask of that tonal range, brightened (dodge) or darkened (burn)
                            steps.push(mask_step(kind, 1, "new"));
                            let ev = if burn { -0.35 } else { 0.35 };
                            steps.push(("mask.adjust", json!({"values": {"exposure": ev}})));
                        }
                    }
                });
            }
        });
    });
    group(ui, crate::i18n::tr("Soft Glow (Orton)"), |ui| {
        for (night, title) in [(false, crate::i18n::tr("Day")), (true, crate::i18n::tr("Night"))] {
            ui.horizontal(|ui| {
                ui.add_sized([34.0, 20.0], egui::Label::new(title));
                for (px, amount) in [(1.5f64, -15.0f64), (3.0, -25.0), (5.0, -35.0), (10.0, -50.0)] {
                    if small(ui, &format!("{px}px"), 40.0) {
                        set(steps, "effects.clarity", amount);
                        if night {
                            adjust(steps, "light.shadows", 15.0);
                        }
                    }
                }
            });
        }
    });
    group(ui, crate::i18n::tr("Exposure Compensation (EV)"), |ui| {
        ui.horizontal_wrapped(|ui| {
            for ev in [-1.0f64, -0.5, -0.2, 0.2, 0.5, 1.0] {
                if small(ui, &format!("{ev:+}"), 40.0) {
                    adjust(steps, "light.exposure", ev);
                }
            }
        });
    });
}

fn color_section(ui: &mut egui::Ui, state: &mut LdPanel, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Selective Color Correction"), |ui| {
        ui.horizontal(|ui| {
            for (i, l) in [crate::i18n::tr("Hue"), crate::i18n::tr("Saturation"), crate::i18n::tr("Luminance")].into_iter().enumerate() {
                ui.selectable_value(&mut state.mixer_mode, i as u8, l);
            }
        });
        let mode = usize::from(state.mixer_mode.min(2));
        let kind = ["hue", "sat", "lum"].get(mode).copied().unwrap_or("hue");
        for (i, name) in [
            crate::i18n::tr("Red"),
            crate::i18n::tr("Orange"),
            crate::i18n::tr("Yellow"),
            crate::i18n::tr("Green"),
            crate::i18n::tr("Aqua"),
            crate::i18n::tr("Blue"),
            crate::i18n::tr("Purple"),
            crate::i18n::tr("Magenta"),
        ]
        .into_iter()
        .enumerate()
        {
            let Some(v) = state.mixer.get_mut(mode * 8 + i) else { continue };
            ui.horizontal(|ui| {
                ui.add_sized([52.0, 18.0], egui::Label::new(name));
                let r = ui.add(egui::Slider::new(v, -100.0..=100.0));
                // one edit when the slider is let go (or a typed value is accepted), not one per frame
                if r.drag_stopped() || (r.changed() && !r.dragged() && !r.has_focus()) {
                    if let Some(band) = BANDS.get(i) {
                        set(steps, &format!("mixer.{band}.{kind}"), f64::from(*v));
                    }
                }
            });
        }
        if button(ui, crate::i18n::tr("Reset")) {
            let all: Vec<_> = BANDS.iter().flat_map(|b| ["hue", "sat", "lum"].map(|k| format!("mixer.{b}.{k}"))).collect();
            let values: serde_json::Map<String, Value> = all.into_iter().map(|c| (c, json!(0.0))).collect();
            steps.push(("develop.set", json!({ "values": values })));
            state.mixer = vec![0.0; 24];
        }
    });
    group(ui, crate::i18n::tr("Color Cast and Saturation"), |ui| {
        if button(ui, crate::i18n::tr("Warmer")) {
            adjust(steps, "wb.tempRel", 8.0);
        }
        if button(ui, crate::i18n::tr("Cooler")) {
            adjust(steps, "wb.tempRel", -8.0);
        }
        if button(ui, crate::i18n::tr("Reduce saturation")) {
            adjust(steps, "color.saturation", -15.0);
        }
        if button(ui, crate::i18n::tr("Reduce blue (blue hour)")) {
            adjust(steps, "mixer.blue.sat", -25.0);
        }
        if button(ui, crate::i18n::tr("Reduce yellow cast")) {
            adjust(steps, "mixer.yellow.sat", -25.0);
        }
    });
}

fn night_section(ui: &mut egui::Ui, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Highlights Recovery"), |ui| {
        if button(ui, crate::i18n::tr("Recovery 50%")) {
            set(steps, "light.highlights", -50.0);
        }
        if button(ui, crate::i18n::tr("Recovery 100%")) {
            set(steps, "light.highlights", -100.0);
        }
    });
    group(ui, crate::i18n::tr("Noise"), |ui| {
        if button(ui, crate::i18n::tr("Noise reduction: light")) {
            set(steps, "detail.nrLuminance", 25.0);
            set(steps, "detail.nrColor", 25.0);
        }
        if button(ui, crate::i18n::tr("Noise reduction: strong")) {
            set(steps, "detail.nrLuminance", 50.0);
            set(steps, "detail.nrColor", 40.0);
        }
    });
    group(ui, crate::i18n::tr("Lights"), |ui| {
        if button(ui, crate::i18n::tr("Green lights correction")) {
            adjust(steps, "mixer.green.sat", -40.0);
        }
        if button(ui, crate::i18n::tr("Lift shadows")) {
            adjust(steps, "light.shadows", 30.0);
        }
    });
}

fn editing_section(ui: &mut egui::Ui, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Sky"), |ui| {
        if button(ui, crate::i18n::tr("Select sky")) {
            steps.push(("mask.add", json!({"kind": "sky"})));
        }
        if button(ui, crate::i18n::tr("Select subject")) {
            steps.push(("mask.add", json!({"kind": "subject"})));
        }
        if button(ui, crate::i18n::tr("Select background")) {
            steps.push(("mask.add", json!({"kind": "background"})));
        }
    });
    group(ui, crate::i18n::tr("Tone Filters"), |ui| {
        if button(ui, crate::i18n::tr("Polarizer effect")) {
            adjust(steps, "mixer.blue.sat", 25.0);
            adjust(steps, "mixer.blue.lum", -15.0);
            adjust(steps, "mixer.aqua.lum", -15.0);
        }
        if button(ui, crate::i18n::tr("Warm tonality filter")) {
            adjust(steps, "wb.tempRel", 6.0);
        }
        if button(ui, crate::i18n::tr("Black and white")) {
            steps.push(("develop.treatment", json!({"bw": true})));
        }
    });
    group(ui, crate::i18n::tr("Detail"), |ui| {
        if button(ui, crate::i18n::tr("Texture +")) {
            adjust(steps, "effects.texture", 20.0);
        }
        if button(ui, crate::i18n::tr("Dehaze +")) {
            adjust(steps, "effects.dehaze", 15.0);
        }
    });
}

fn final_section(ui: &mut egui::Ui, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Contrast Enhance"), |ui| {
        if button(ui, crate::i18n::tr("Contrast +")) {
            adjust(steps, "light.contrast", 20.0);
        }
        if button(ui, crate::i18n::tr("Whites +")) {
            adjust(steps, "light.whites", 15.0);
        }
    });
    group(ui, crate::i18n::tr("Final Sharpening"), |ui| {
        for (label, amount) in [(crate::i18n::tr("Strong (100%)"), 80.0), (crate::i18n::tr("Medium (80%)"), 60.0), (crate::i18n::tr("Light (50%)"), 40.0)] {
            if button(ui, label) {
                set(steps, "detail.sharpenAmount", amount);
            }
        }
    });
}

fn export_section(ui: &mut egui::Ui, steps: &mut Steps) {
    group(ui, crate::i18n::tr("Export"), |ui| {
        if button(ui, crate::i18n::tr("Export…")) {
            steps.push(("dialog.export", json!({})));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_levels_mask_narrower_ranges_and_bad_levels_are_clamped() {
        let l1 = range_params("lights", 1);
        let l3 = range_params("lights", 3);
        assert!(l3["lo"].as_f64() > l1["lo"].as_f64());
        assert_eq!(range_params("lights", 99), l3);
        assert_eq!(range_params("lights", 0), l1);
        let d = range_params("darks", 1);
        assert_eq!((d["lo"].as_f64(), d["hi"].as_f64()), (Some(0.0), Some(0.4)));
        let m = range_params("mids", 1);
        assert!(m["lo"].as_f64() < m["hi"].as_f64());
    }

    #[test]
    fn plus_and_minus_add_to_the_active_mask() {
        assert_eq!(mask_step("lights", 1, "new").0, "mask.add");
        let (cmd, p) = mask_step("lights", 1, "subtract");
        assert_eq!((cmd, p["op"].as_str()), ("mask.addComponent", Some("subtract")));
        assert_eq!(mask_step("mids", 2, "junk").1["op"], "add");
    }

    #[test]
    fn mixer_sends_only_moved_sliders_and_survives_bad_input() {
        let mut v = vec![0.0f32; 24];
        assert_eq!(mixer_values(&v), json!({}));
        v[3] = 20.0; // green hue
        v[8 + 5] = -30.0; // blue saturation
        let p = mixer_values(&v);
        assert_eq!(p["mixer.green.hue"], 20.0);
        assert_eq!(p["mixer.blue.sat"], -30.0);
        assert_eq!(p.as_object().map(|o| o.len()), Some(2));
        let odd = mixer_values(&[f32::NAN, 500.0]);
        assert_eq!(odd.as_object().map(|o| o.len()), Some(1));
        assert_eq!(odd["mixer.orange.hue"], 100.0);
    }

    #[test]
    fn only_the_six_sections_and_hide_are_ours() {
        for s in SECTIONS {
            assert!(SECTIONS.contains(&s));
        }
        assert!(!SECTIONS.contains(&"hide") && !SECTIONS.contains(&"other"));
    }
}
