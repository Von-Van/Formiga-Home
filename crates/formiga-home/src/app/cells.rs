//! The household's cells: a small card in the corner of the house with a face for everyone the
//! house was opened with, lit for whoever is in. A click sends someone out to the desktop, where
//! Desktop has them back in the colony, or has them in again. A resident kept out stays out the
//! next time the house opens; a friend comes or not with each visit.

use super::notebook::ink;
use super::*;
use crate::life::Inside;

impl HomeApp {
    /// Who is in the house just now, said to Desktop whenever it changes.
    pub(super) fn say_who_is_indoors(&mut self) {
        if !self.host.follows_indoors() {
            return;
        }
        let indoors = self.life.indoors();
        if self.indoors_said.as_ref() == Some(&indoors) {
            return;
        }
        let travelers: Vec<TravelerId> = indoors.iter().copied().map(TravelerId).collect();
        if let Err(error) = self.host.say_indoors(&travelers) {
            eprintln!("formiga-home: {error:#}");
        }
        self.indoors_said = Some(indoors);
    }

    /// Send someone in the house out to the desktop, or have someone out on it in.
    pub(super) fn toggle_inside(&mut self, id: Id) {
        let now = self.now();
        let here = self.life.inside(id) == Some(Inside::Here);
        if self.carried == Some(id) {
            self.life.put_down(&self.house, id, now);
            self.carried = None;
        }
        let changed = if here {
            self.life.send_out(&self.house, id, now)
        } else {
            self.life.bring_in(&self.household, &self.house, id, now)
        };
        if !changed {
            return;
        }
        if here && self.selected == Some(id) {
            self.selected = None;
            self.menu = None;
        }
        if !self.household.is_visitor(id)
            && let Some(home) = self.state.household_mut(self.keeper)
        {
            home.stays_out.retain(|out| out.0 != id);
            if here {
                home.stays_out.push(TravelerId(id));
            }
            self.keep();
        }
        let name = self.name(id);
        self.say(if here {
            format!("{name} is off out to the desktop.")
        } else {
            format!("{name} is coming in.")
        });
    }

    /// The card of cells, along the foot of the house's page: residents first, then friends.
    pub(super) fn cells(&mut self, ui: &mut egui::Ui, page: egui::Rect, unit: f32) {
        if self.mode != Mode::Live || !self.host.follows_indoors() {
            return;
        }
        let ctx = ui.ctx().clone();
        let dark = ui.visuals().dark_mode;
        let everyone: Vec<Id> = self.household.everyone().map(|r| r.id).collect();
        // As wide as a face, or narrower for a big household, so the card stops short of the
        // zoom's card in the other corner.
        let room = page.width() - 54.0 * unit;
        let side = (room / everyone.len().max(1) as f32).clamp(12.0 * unit, 20.0 * unit);
        let cell = egui::vec2(side, side);
        let card = egui::Rect::from_min_size(
            egui::pos2(page.min.x + 3.0 * unit, page.max.y - cell.y - 3.0 * unit),
            egui::vec2(cell.x * everyone.len() as f32, cell.y),
        );
        let painter = ui.painter().clone();
        painter.rect_filled(card.shrink(unit), 0.0, ink::card(dark));
        pages::stepped(&painter, card, unit, ink::line(dark).gamma_multiply(0.7));
        for (index, id) in everyone.into_iter().enumerate() {
            let rect =
                egui::Rect::from_min_size(card.min + egui::vec2(cell.x * index as f32, 0.0), cell);
            let inside = self.life.inside(id).unwrap_or(Inside::Out);
            let here = inside == Inside::Here;
            let name = self.name(id);
            let hint = match inside {
                Inside::Here if self.household.is_visitor(id) => {
                    format!("{name} is visiting. Click to send {name} back out to the desktop.")
                }
                Inside::Here => format!("{name} is home. Click to send {name} out to the desktop."),
                Inside::Coming => {
                    format!(
                        "{name} is on the way over, out on the desktop. Click to have {name} in now."
                    )
                }
                Inside::GoneHome => {
                    format!("{name} has gone home, out on the desktop. Click to have {name} back.")
                }
                Inside::Out => format!("{name} is out on the desktop. Click to have {name} in."),
            };
            let response = ui
                .interact(rect, egui::Id::new(("cell", id)), egui::Sense::click())
                .on_hover_text(hint)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if here || response.hovered() {
                painter.rect_filled(
                    rect.shrink(unit),
                    0.0,
                    ink::mint(dark).gamma_multiply(if response.hovered() { 1.0 } else { 0.6 }),
                );
            }
            if let Some((texture, pixels)) = self.portrait(&ctx, id) {
                let shown = pixels * unit;
                let at = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.center().x - shown.x / 2.0,
                        rect.bottom() - unit - shown.y,
                    ),
                    shown,
                );
                // Whoever is out is a faded face: still there to be had in.
                let tint = if here {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_white_alpha(90)
                };
                painter.with_clip_rect(rect.shrink(unit)).image(
                    texture,
                    at,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    tint,
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, here, &name)
            });
            if response.clicked() {
                self.toggle_inside(id);
            }
        }
    }
}
