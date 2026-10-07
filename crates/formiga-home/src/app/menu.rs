//! The menu over the room, of what can be asked there, and what choosing from it does.

use super::*;

/// What can be chosen from the menu over the room.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Entry {
    Ask(Act),
    Pet,
    Choose(Id),
    /// Ask a visitor to stay over.
    StayOver(Id),
    /// Ask a visitor to come and live here.
    MoveIn(Id),
}

pub(super) struct Menu {
    pub(super) at: egui::Pos2,
    pub(super) title: String,
    pub(super) entries: Vec<(Entry, String)>,
    pub(super) opened: f32,
}

impl HomeApp {
    pub(super) fn menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = &self.menu else { return };
        let mut chosen = None;
        let mut close = false;
        let unit = notebook::unit(ctx.pixels_per_point());
        let area = egui::Area::new(egui::Id::new("actions"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.at + egui::vec2(8.0, 8.0))
            .constrain(true)
            .show(ctx, |ui| {
                let dark = ui.visuals().dark_mode;
                // A card of choices, with the notebook's stepped edge and a shadow under it.
                let shadow = ui.painter().add(egui::Shape::Noop);
                let shown = egui::Frame::new()
                    .fill(notebook::ink::card(dark))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        // As wide as its longest choice, and every row that wide.
                        let widest = menu
                            .entries
                            .iter()
                            .map(|(_, label)| {
                                ui.painter()
                                    .layout_no_wrap(
                                        label.clone(),
                                        egui::TextStyle::Button.resolve(ui.style()),
                                        egui::Color32::WHITE,
                                    )
                                    .size()
                                    .x
                            })
                            .fold(150.0_f32, f32::max);
                        ui.set_width(widest + 12.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if !menu.title.is_empty() {
                            notebook::kicker(ui, &menu.title);
                            ui.add_space(3.0);
                        }
                        for (entry, label) in &menu.entries {
                            let row = egui::Button::new(label)
                                .frame(false)
                                .min_size(egui::vec2(ui.available_width(), 20.0));
                            if ui.add(row).clicked() {
                                chosen = Some(entry.clone());
                            }
                        }
                        ui.add_space(3.0);
                        let never = egui::Button::new(
                            egui::RichText::new("Never mind")
                                .italics()
                                .color(notebook::ink::muted(dark)),
                        )
                        .frame(false);
                        if ui.add(never).clicked() {
                            close = true;
                        }
                    });
                let rect = shown.response.rect;
                ui.painter().set(
                    shadow,
                    egui::Shape::rect_filled(
                        rect.translate(egui::vec2(2.0 * unit, 2.0 * unit)),
                        0.0,
                        egui::Color32::from_black_alpha(60),
                    ),
                );
                pages::stepped(
                    ui.painter(),
                    rect.expand(unit),
                    unit,
                    notebook::ink::line(dark),
                );
            });
        let clicked_elsewhere = ctx.input(|input| input.pointer.any_click())
            && !area.response.contains_pointer()
            && self.now() - menu.opened > 0.2;
        if let Some(entry) = chosen {
            self.menu = None;
            let Some(chosen_id) = self.selected else {
                return;
            };
            match entry {
                Entry::Ask(act) => self.ask(chosen_id, act),
                Entry::Pet => {
                    let now = self.now();
                    self.life.pet(&self.household, chosen_id, now);
                }
                Entry::Choose(other) => self.selected = Some(other),
                Entry::MoveIn(visitor) => self.ask_to_move_in(visitor),
                Entry::StayOver(visitor) => {
                    if !self.life.ask_to_stay(&self.household, visitor) {
                        self.say(format!(
                            "{} would rather go home tonight.",
                            self.name(visitor)
                        ));
                    }
                }
            }
        } else if close || clicked_elsewhere {
            self.menu = None;
        }
    }
}
