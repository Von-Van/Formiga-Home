//! What is written on the notes page. Living in the house, it is who is home: a small portrait
//! and a name each, and only for whoever is chosen a little more, what it is doing, what it has
//! come to like, and what it has been asked. Arranging, it is the found things, the furniture
//! and the rooms, each a page of small pictures rather than a list, with what each one is kept
//! for when it is pointed at.

use super::lines;
use super::notebook::{LABEL, ink, kicker, label_job};
use super::*;
use crate::art::{displays, shell};
use formiga_art::{BodyClip, CreatureRenderer, EyelidPose, FaceRenderState, GazeDirection};
use formiga_core::ActionKind;
use formiga_home_contract::DisplayItem;

/// How big a picture can be drawn inside `fit` points, whole screen pixels to each of its own,
/// and never more than `most` of them.
pub(super) fn crisp(ui: &egui::Ui, pixels: egui::Vec2, fit: egui::Vec2, most: f32) -> egui::Vec2 {
    let ppp = ui.ctx().pixels_per_point();
    let across = (fit.x * ppp / pixels.x.max(1.0)).min(fit.y * ppp / pixels.y.max(1.0));
    pixels * across.floor().clamp(1.0, most) / ppp
}

/// A picture cut down to what is drawn in it, so it is shown as big as its tile allows.
pub(super) fn cropped(canvas: Canvas) -> Canvas {
    let Some((l, t, r, b)) = canvas.alpha_bounds() else {
        return canvas;
    };
    let (w, h) = (r - l + 1, b - t + 1);
    let mut cut = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            cut.set(
                x as i32,
                y as i32,
                canvas.get((l + x) as i32, (t + y) as i32),
            );
        }
    }
    cut
}

/// The notes page's ruled lines, for writing whose top is at `top` (above the page once it has
/// scrolled), drawn only within `within`: a line every `RULE_PITCH` of the notebook's pixels down
/// the writing, each a row of the notebook's pixels on whole screen pixels, so the lines stay
/// crisp and stay with the words they are under.
fn ruling(ui: &egui::Ui, page: egui::Rect, top: f32, within: egui::Rect, unit: f32) -> egui::Shape {
    use crate::art::notebook::{FIRST_RULE, RULE_PITCH, palette, rule_runs};
    let colour = palette(ui.visuals().dark_mode).rule;
    let colour = egui::Color32::from_rgba_unmultiplied(colour.r, colour.g, colour.b, colour.a);
    let ppp = ui.ctx().pixels_per_point();
    let width = (page.width() / unit).round() as i32;
    let pitch = RULE_PITCH as f32 * unit;
    let first = top + FIRST_RULE as f32 * unit;
    let mut line = ((within.top() - first) / pitch).ceil().max(0.0) as i32;
    let mut mesh = egui::Mesh::default();
    loop {
        let y = ((first + line as f32 * pitch) * ppp).round() / ppp;
        if y + unit > within.bottom() {
            break;
        }
        for (start, length) in rule_runs(line, width) {
            mesh.add_colored_rect(
                egui::Rect::from_min_size(
                    egui::pos2(page.left() + start as f32 * unit, y),
                    egui::vec2(length as f32 * unit, unit),
                ),
                colour,
            );
        }
        line += 1;
    }
    egui::Shape::mesh(mesh)
}

/// What was asked of a thing from its menu on the found things page.
enum Tapped {
    PutAway(DisplayId),
    LetGo(DisplayId),
}

/// A one-unit outline round `rect` with its corners left out: the notebook's stepped edge.
pub(super) fn stepped(painter: &egui::Painter, rect: egui::Rect, unit: f32, colour: egui::Color32) {
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    for strip in [
        egui::Rect::from_min_max(egui::pos2(l + unit, t), egui::pos2(r - unit, t + unit)),
        egui::Rect::from_min_max(egui::pos2(l + unit, b - unit), egui::pos2(r - unit, b)),
        egui::Rect::from_min_max(egui::pos2(l, t + unit), egui::pos2(l + unit, b - unit)),
        egui::Rect::from_min_max(egui::pos2(r - unit, t + unit), egui::pos2(r, b - unit)),
    ] {
        painter.rect_filled(strip, 0.0, colour);
    }
}

/// A square of the page to put a picture in, which can be clicked and dragged: a slip of card
/// with a stepped edge, lit when it is pointed at or chosen.
pub(super) fn tile(
    ui: &mut egui::Ui,
    id: egui::Id,
    size: egui::Vec2,
    chosen: bool,
    unit: f32,
) -> (egui::Response, egui::Rect) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    let dark = ui.visuals().dark_mode;
    let fill = if chosen || response.hovered() {
        ink::mint(dark)
    } else {
        ink::card(dark)
    };
    let painter = ui.painter();
    painter.rect_filled(rect.shrink(unit), 0.0, fill);
    let edge = if chosen {
        ink::forest(dark)
    } else {
        ink::line(dark).gamma_multiply(0.55)
    };
    stepped(painter, rect, unit, edge);
    (response, rect)
}

impl HomeApp {
    /// The notes page, as it is for the mode and the page turned to, written on the ruled
    /// `page`. Each page keeps how far it was scrolled, and its ruled lines scroll with it.
    pub(super) fn notes(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        page: egui::Rect,
        unit: f32,
    ) {
        // The ruling goes under the writing, so its place is kept before the writing is laid
        // out, and filled once the scroll says where the writing is.
        let ruled = egui::Rect::from_min_max(
            egui::pos2(page.left(), ui.max_rect().top()),
            egui::pos2(page.right(), page.bottom() - 4.0 * unit),
        );
        let painter = ui.ctx().layer_painter(ui.layer_id()).with_clip_rect(ruled);
        let slot = painter.add(egui::Shape::Noop);
        let scrolled = egui::ScrollArea::vertical()
            .id_salt(("notes", self.mode as u8, self.page() as u8))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // What is written on the page finds its ruled lines through the grid.
                use crate::art::notebook::{FIRST_RULE, RULE_PITCH};
                lines::RuleGrid {
                    top: ui.max_rect().top() + (FIRST_RULE - RULE_PITCH) as f32 * unit,
                    pitch: RULE_PITCH as f32 * unit,
                }
                .keep(ui.ctx());
                match self.mode {
                    Mode::Live if self.live_page == Drawer::Journal => self.journal_page(ui, unit),
                    Mode::Live if self.live_page == Drawer::Projects => {
                        self.projects_page(ui, ctx, unit);
                    }
                    Mode::Live => self.household_page(ui, ctx, unit),
                    Mode::Arrange => {
                        match self.drawer {
                            Drawer::Furniture => self.furniture_page(ui, ctx, unit),
                            Drawer::Room => self.rooms_page(ui, ctx, unit),
                            Drawer::Designs => self.designs_page(ui, unit),
                            _ => self.finds_page(ui, ctx, unit),
                        }
                        ui.add_space(10.0);
                        self.arranging_foot(ui);
                    }
                }
            });
        lines::RuleGrid::forget(ui.ctx());
        let top = scrolled.inner_rect.top() - scrolled.state.offset.y;
        painter.set(slot, ruling(ui, page, top, ruled, unit));
    }

    /// A resident's face for the notes: its idle frame, cut down to its head and shoulders.
    pub(super) fn portrait(
        &mut self,
        ctx: &egui::Context,
        id: Id,
    ) -> Option<(egui::TextureId, egui::Vec2)> {
        let resident = self.household.resident(id)?;
        let (genome, dress, face) = (
            resident.genome().clone(),
            resident.dress,
            resident.character.idle_face(),
        );
        let still = self.household.reduce_motion();
        Some(self.thumbnail(ctx, format!("portrait:{id}"), move || {
            let frame = CreatureRenderer::render_dressed_composited_frame(
                &genome,
                dress,
                BodyClip::Action(ActionKind::Idle),
                0,
                true,
                still,
                FaceRenderState {
                    expression: face,
                    eyelids: EyelidPose::Open,
                    gaze: GazeDirection::default(),
                },
            );
            let Some((l, t, r, b)) = frame.alpha_bounds() else {
                return frame;
            };
            let (w, h) = (r - l + 1, (b - t + 1).min(24));
            let mut cut = Canvas::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    cut.set(
                        x as i32,
                        y as i32,
                        frame.get((l + x) as i32, (t + y) as i32),
                    );
                }
            }
            cut
        }))
    }

    /// The household's journal, newest first, a line a moment under the day it happened.
    fn journal_page(&mut self, ui: &mut egui::Ui, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let journal = home_of(&self.state, self.keeper).journal.clone();
        kicker(ui, "Journal");
        if journal.is_empty() {
            lines::write(
                ui,
                egui::RichText::new(
                    "Nothing written yet. Friends who come over, keepsakes, new rooms and \
                     favourites are noted here.",
                )
                .italics()
                .color(ink::muted(dark)),
            );
            return;
        }
        let today = crate::journal::day_of(time::OffsetDateTime::now_utc());
        let mut last = None;
        for entry in journal.iter().rev() {
            let day = crate::journal::day_of(entry.at_utc);
            if last != Some(day) {
                if last.is_some() {
                    // A line left blank between days.
                    ui.add_space(3.0 * unit);
                }
                lines::write(
                    ui,
                    egui::RichText::new(crate::journal::heading(day, today))
                        .small()
                        .color(ink::forest(dark)),
                );
                last = Some(day);
            }
            lines::write(
                ui,
                crate::journal::line(&entry.moment, &self.household.snapshot),
            );
        }
    }

    /// The other houses of the village, to go over to.
    fn next_door_links(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if !self.host.goes_next_door() {
            return;
        }
        let here = self.household.snapshot.household.keeper;
        let houses: Vec<(TravelerId, String)> = self
            .household
            .snapshot
            .village
            .iter()
            .filter(|house| house.keeper != here)
            .map(|house| (house.keeper, format!("{}'s house", house.name)))
            .collect();
        if houses.is_empty() {
            return;
        }
        kicker(ui, "Next door");
        let dark = ui.visuals().dark_mode;
        let mut going = None;
        lines::ruled_row(ui, |ui| {
            // A house's name moves to the next line whole rather than breaking.
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.x = 10.0;
            for (keeper, name) in &houses {
                let link = lines::write_with(
                    ui,
                    egui::RichText::new(name.as_str())
                        .underline()
                        .color(ink::forest(dark)),
                    egui::Sense::click(),
                )
                .on_hover_text(format!("Go over to {name}"));
                if link.clicked() {
                    going = Some(*keeper);
                }
            }
        });
        ui.add_space(8.0);
        if let Some(keeper) = going {
            self.go_next_door(ctx, keeper);
        }
    }

    fn household_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let present = self.life.present();
        let residents: Vec<Id> = self
            .household
            .residents
            .iter()
            .map(|r| r.id)
            .filter(|id| present.contains(id))
            .collect();
        let visiting: Vec<Id> = self
            .household
            .visitors
            .iter()
            .map(|r| r.id)
            .filter(|id| present.contains(id))
            .collect();
        for (heading, ids) in [("At home", residents), ("Visiting", visiting)] {
            if ids.is_empty() {
                continue;
            }
            kicker(ui, heading);
            for id in ids {
                self.resident_row(ui, ctx, id, unit);
                if self.selected == Some(id) {
                    self.chosen_card(ui, id, unit);
                }
            }
            ui.add_space(8.0);
        }
        self.next_door_links(ui, ctx);
        let hint = match self.selected {
            Some(_) => "Click something in the house to see what they could do.",
            None => "Choose someone, then something in the house.",
        };
        lines::write(
            ui,
            egui::RichText::new(hint)
                .small()
                .italics()
                .color(ink::muted(dark)),
        );
        self.help_button(ui);
    }

    /// A small "how to" at the foot of the page, which opens the sticky note of everything the
    /// mouse and keys can do.
    fn help_button(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let word = if self.help {
            "Put the note away"
        } else {
            "How to\u{2026}"
        };
        let response = lines::write_with(
            ui,
            egui::RichText::new(word)
                .small()
                .underline()
                .color(ink::forest(dark)),
            egui::Sense::click(),
        );
        if response.clicked() {
            self.help = !self.help;
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    /// The sticky note of everything the mouse and keys can do, taped over the foot of the notes.
    pub(super) fn help_note(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        notes: egui::Rect,
        unit: f32,
    ) {
        if !self.help {
            return;
        }
        let dark = ui.visuals().dark_mode;
        let undo = if cfg!(target_os = "macos") {
            "\u{2318}Z"
        } else {
            "Ctrl+Z"
        };
        let text = match self.mode {
            Mode::Live => {
                let cells = if self.host.follows_indoors() {
                    "The faces under the house are who is in: click one to send them out to the \
                     desktop, or back in. "
                } else {
                    ""
                };
                format!(
                    "Click someone, then a seat, a toy, a find or someone else, to see what they \
                     could do there. Click the floor to send them over, drag them to carry them, \
                     and right-click for a pat. Up to three things can be asked at once.\n\n\
                     {cells}The project board, the next page, is what the household can make of \
                     its home.\n\n\
                     A pinch or + and \u{2212} come closer, and a drag across the floor moves \
                     about. N chooses the next one, [ and ] turn the pages, A arranges, P saves a \
                     picture, and H puts this note away."
                )
            }
            Mode::Arrange => {
                format!(
                    "Drag or click something on these pages to pick it up, and click to put it \
                     down. A doorway slides along its wall. R or a right-click turns a piece, \
                     Delete puts it away, and {undo} undoes.\n\n\
                     A keepsake can be let go from its right-click menu. Designs keeps the \
                     house, or a room of it, to try again or share.\n\n\
                     [ and ] turn the pages, L goes back to living in the house, and H puts this \
                     note away. Everyone waits while the house is arranged."
                )
            }
        };
        let width = notes.width().min(260.0);
        let galley = ui.painter().layout(
            text,
            egui::FontId::proportional(12.5),
            ink::page(dark),
            width - 24.0,
        );
        let size = galley.size() + egui::vec2(24.0, 26.0);
        let rect =
            egui::Rect::from_min_size(egui::pos2(notes.max.x - size.x, notes.max.y - size.y), size);
        let pixels = ((size.x / unit).ceil() as i32, (size.y / unit).ceil() as i32);
        let key = format!("note:{}x{}:{dark}", pixels.0, pixels.1);
        let (texture, _) = self.thumbnail(ctx, key, || crate::art::notebook::note(pixels, dark));
        let painted = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(pixels.0 as f32 + 2.0, pixels.1 as f32 + 2.0) * unit,
        );
        let painter = ui.painter();
        painter.image(
            texture,
            painted,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.galley(rect.min + egui::vec2(12.0, 16.0), galley, ink::page(dark));
        // Clicking the note puts it away.
        let response = ui.interact(rect, egui::Id::new("help note"), egui::Sense::click());
        if response.clicked() {
            self.help = false;
        }
    }

    /// A resident's line in the notes: its portrait and its name, to choose it by.
    fn resident_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, id: Id, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let Some(resident) = self.household.resident(id) else {
            return;
        };
        let name = resident.name.clone();
        let little = resident.is_little();
        let visitor = self.household.is_visitor(id);
        // Two lines of the page: the portrait stands on the lower one, and the name rests on it.
        let rect = lines::band(ui, 2.0, 22.0 * unit);
        let response = ui.interact(rect, egui::Id::new(("resident", id)), egui::Sense::click());
        let chosen = self.selected == Some(id);
        if chosen {
            ui.painter()
                .rect_filled(rect, 0.0, ink::mint(dark).gamma_multiply(0.55));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, ink::mint(dark).gamma_multiply(0.3));
        }
        if let Some((texture, pixels)) = self.portrait(ctx, id) {
            let frame = egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - 22.0 * unit),
                egui::pos2(rect.left() + 22.0 * unit, rect.bottom()),
            );
            let shown = pixels * unit;
            let at = egui::Rect::from_min_size(
                egui::pos2(frame.center().x - shown.x / 2.0, frame.bottom() - shown.y),
                shown,
            );
            ui.painter().with_clip_rect(frame).image(
                texture,
                at,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        let text_left = rect.min.x + 26.0 * unit;
        let galley = ui.painter().layout_no_wrap(
            name.clone(),
            egui::FontId::proportional(14.0),
            ink::page(dark),
        );
        let y = lines::on_rule(&galley, rect.bottom());
        let name_width = galley.size().x;
        ui.painter()
            .galley(egui::pos2(text_left, y), galley, ink::page(dark));
        let aside = if visitor && self.life.staying(id) {
            Some("staying over".to_owned())
        } else if visitor {
            self.household
                .friend_of(id)
                .map(|friend| format!("{}'s friend", friend.name))
        } else if little {
            Some("little one".to_owned())
        } else {
            None
        };
        if let Some(aside) = aside {
            let galley = ui.painter().layout_no_wrap(
                aside,
                egui::FontId::proportional(11.5),
                ink::muted(dark),
            );
            let y = lines::on_rule(&galley, rect.bottom());
            ui.painter().galley(
                egui::pos2(text_left + name_width + 6.0, y),
                galley,
                ink::muted(dark),
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, &name)
        });
        if response.clicked() {
            self.selected = if chosen { None } else { Some(id) };
            self.menu = None;
        }
    }

    /// A little more about whoever is chosen: its nature, what it is doing, what it has come to
    /// like, and what it has been asked.
    fn chosen_card(&mut self, ui: &mut egui::Ui, id: Id, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let house = self.house.clone();
        let snapshot = self.household.snapshot.clone();
        let Some(resident) = self.household.resident(id) else {
            return;
        };
        let phrase = resident.traveler.character.phrase.clone();
        let doing = self.life.doing(&self.household, &house, &snapshot, id);
        let likings = home_of(&self.state, self.keeper).likings.clone();
        let liked: Vec<String> = life::favourites(&likings, &house, id)
            .iter()
            .map(|(_, thing)| life::name_of(&house, &snapshot, thing))
            .collect();
        let gap = 26.0 * unit;
        ui.horizontal(|ui| {
            ui.add_space(gap);
            ui.vertical(|ui| self.about(ui, id, dark, &phrase, doing, &liked));
        });
        ui.add_space(6.0);
    }

    fn about(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        dark: bool,
        phrase: &str,
        doing: String,
        liked: &[String],
    ) {
        let now = self.now();
        let house = self.house.clone();
        let snapshot = self.household.snapshot.clone();
        {
            lines::write(
                ui,
                egui::RichText::new(phrase)
                    .size(12.0)
                    .italics()
                    .color(ink::muted(dark)),
            );
            if !doing.is_empty() {
                lines::write(
                    ui,
                    egui::RichText::new(self.in_which_room(id, doing)).size(12.0),
                );
            }
            if !liked.is_empty() {
                lines::write(
                    ui,
                    egui::RichText::new(format!("Likes {}", and_list(liked)))
                        .size(12.0)
                        .color(ink::muted(dark)),
                );
            }
            let queue = self.life.queue(id);
            if !queue.is_empty() {
                lines::write(ui, label_job("Asked", LABEL, ink::forest(dark)));
                let mut cancel = None;
                for (position, act) in queue.iter().enumerate() {
                    lines::ruled_row(ui, |ui| {
                        if ui
                            .small_button("\u{2715}")
                            .on_hover_text("Take this back")
                            .clicked()
                        {
                            cancel = Some(position);
                        }
                        lines::write(
                            ui,
                            egui::RichText::new(act.label(&self.household, &house, &snapshot))
                                .size(12.0),
                        );
                    });
                }
                if let Some(position) = cancel {
                    self.life.cancel(id, position, now);
                }
            }
        }
    }

    /// What someone is doing, and in a house of rooms, in which: "in the nook".
    fn in_which_room(&self, id: Id, doing: String) -> String {
        if self.house.rooms.len() < 2 {
            return doing;
        }
        let Some(room) = self
            .life
            .actors
            .iter()
            .find(|actor| actor.id == id && !actor.hidden)
            .and_then(|actor| self.house.room_of_point(actor.pos))
        else {
            return doing;
        };
        let name = catalog::room_name(self.house.rooms[usize::from(room)].kind.as_ref(), room == 0)
            .to_lowercase();
        format!("{} In the {name}.", doing)
    }

    fn finds_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let snapshot = self.household.snapshot.clone();
        let (colony, made): (Vec<&DisplayItem>, Vec<&DisplayItem>) = snapshot
            .inventory
            .iter()
            .partition(|item| !keepsakes::is_keepsake(item));
        let house = self.house.clone();
        let mut chosen = None;
        if colony.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Nothing found yet. Whatever the colony finds will be here to show.",
                )
                .italics()
                .color(ink::muted(dark)),
            );
        } else {
            let here = colony
                .iter()
                .filter(|item| {
                    matches!(
                        self.household.whereabouts(&self.state, &item.id),
                        Whereabouts::Here
                    )
                })
                .count();
            kicker(ui, "Found things");
            ui.label(
                egui::RichText::new(format!(
                    "{} the colony has \u{b7} {here} shown here",
                    colony.len()
                ))
                .small()
                .color(ink::muted(dark)),
            );
            ui.add_space(4.0);
            chosen = chosen.or(self.thing_tiles(ui, ctx, &colony, &house, unit));
        }
        if !made.is_empty() {
            ui.add_space(8.0);
            kicker(ui, "Made at home");
            ui.label(
                egui::RichText::new("Left by friends, drawn, or framed, and kept here")
                    .small()
                    .color(ink::muted(dark)),
            );
            // The drawer holds only so many: say how many, and how to make room.
            let most = formiga_home_contract::limits::MAX_MEMENTOS;
            let kept = if made.len() >= most {
                format!("All {most} the drawer holds: let one go to make room for more")
            } else {
                format!(
                    "{} of {most} \u{b7} right-click one to let it go",
                    made.len()
                )
            };
            ui.label(
                egui::RichText::new(kept)
                    .small()
                    .italics()
                    .color(if made.len() >= most {
                        ink::stamp(dark)
                    } else {
                        ink::muted(dark)
                    }),
            );
            ui.add_space(4.0);
            chosen = chosen.or(self.thing_tiles(ui, ctx, &made, &house, unit));
        }
        match chosen {
            Some(Tapped::PutAway(item)) => {
                self.arranging.carrying = Some(Carry::Thing(item));
                if self
                    .arranging
                    .put_away(&mut self.state, self.keeper, &self.house)
                {
                    self.changed("Back in the drawer.");
                }
            }
            Some(Tapped::LetGo(item)) => self.let_keepsake_go(item),
            None => {}
        }
    }

    /// Let one of the household's keepsakes go, as one change undo can take back.
    fn let_keepsake_go(&mut self, item: DisplayId) {
        self.arranging.remember(&self.state);
        if let Some(home) = self.state.household_mut(self.keeper) {
            keepsakes::let_go(home, &item);
        }
        if self.arranging.carrying == Some(Carry::Thing(item)) {
            self.arranging.carrying = None;
        }
        self.keepsakes_changed();
        self.changed("Let go.");
    }

    /// A tile for each of `items`, to pick one up by, and what was asked of one from its menu.
    fn thing_tiles(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        items: &[&DisplayItem],
        house: &House,
        unit: f32,
    ) -> Option<Tapped> {
        let dark = ui.visuals().dark_mode;
        let mut tapped = None;
        let size = egui::vec2(24.0 * unit, 24.0 * unit);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
            for item in items {
                let whereabouts = self.household.whereabouts(&self.state, &item.id);
                let carried = self.arranging.carrying == Some(Carry::Thing(item.id.clone()));
                let (response, rect) = tile(
                    ui,
                    egui::Id::new(("find", item.id.as_str())),
                    size,
                    carried,
                    unit,
                );
                let icon = displays::tile_picture(self.scene.icon(item), item);
                let (texture, pixels) =
                    self.thumbnail(ctx, format!("item:{}", item.id), || cropped(icon));
                let shown = crisp(ui, pixels, size - egui::vec2(6.0, 6.0) * unit, 4.0);
                ui.painter().image(
                    texture,
                    egui::Rect::from_center_size(rect.center(), shown),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                // A red dot for what is shown here; a hollow one for what is shown elsewhere.
                let dot = egui::Rect::from_min_size(
                    rect.right_top() + egui::vec2(-5.0 * unit, 2.0 * unit),
                    egui::vec2(3.0 * unit, 3.0 * unit),
                );
                match &whereabouts {
                    Whereabouts::Here => {
                        ui.painter().rect_filled(dot, 0.0, ink::stamp(dark));
                    }
                    Whereabouts::Elsewhere(_) => {
                        stepped(ui.painter(), dot, unit, ink::muted(dark));
                    }
                    Whereabouts::Nowhere => {}
                }
                let where_now = match &whereabouts {
                    Whereabouts::Nowhere => "Not shown anywhere yet".to_owned(),
                    Whereabouts::Here => format!("Here, {}", where_here(house, &item.id)),
                    Whereabouts::Elsewhere(house) => format!("In {house}"),
                };
                let favourite = self.favourite_of(&Liked::Shown {
                    item: item.id.clone(),
                });
                let name = item.name.clone();
                let keepsake = keepsakes::is_keepsake(item);
                // A keepsake's name already says whom it came from.
                let finder = item.finder_name.clone().filter(|_| !keepsake);
                let response = response.on_hover_ui(|ui| {
                    ui.strong(&name);
                    ui.label(egui::RichText::new(&where_now).small());
                    if let Some(finder) = &finder {
                        ui.label(egui::RichText::new(format!("Found by {finder}")).small());
                    }
                    if let Some(whose) = &favourite {
                        ui.label(egui::RichText::new(whose).small());
                    }
                });
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                });
                if response.clicked() || response.drag_started() {
                    self.arranging.carrying = Some(Carry::Thing(item.id.clone()));
                    self.arranging.dragged = response.drag_started();
                }
                let response = response.on_hover_cursor(egui::CursorIcon::Grab);
                let here = matches!(whereabouts, Whereabouts::Here);
                if here || keepsake {
                    response.context_menu(|ui| {
                        if here && ui.button("Put away").clicked() {
                            tapped = Some(Tapped::PutAway(item.id.clone()));
                            ui.close();
                        }
                        if keepsake && ui.button("Let it go").clicked() {
                            tapped = Some(Tapped::LetGo(item.id.clone()));
                            ui.close();
                        }
                    });
                }
            }
        });
        tapped
    }

    fn furniture_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let made = home_of(&self.state, self.keeper).projects.clone();
        let snapshot = &self.household.snapshot;
        let now = catalog::Arrived {
            made: &made,
            ..catalog::Arrived::of(snapshot, home_of(&self.state, self.keeper))
        };
        let house = self.house.clone();
        let size = egui::vec2(30.0 * unit, 30.0 * unit);
        // The sets, to look at one at a time.
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.set_shown, None, "All");
            for set in catalog::Set::ALL {
                let hint = if set == catalog::Set::Made {
                    "Made by the household, as it finishes its projects".to_owned()
                } else {
                    set.arrives()
                        .when(&now)
                        .unwrap_or_else(|| "Here to use".to_owned())
                };
                ui.selectable_value(&mut self.set_shown, Some(set), set.name())
                    .on_hover_text(hint);
            }
        });
        ui.add_space(4.0);
        let shown = self.set_shown;
        for family in Family::ALL {
            // What the household could make is all on its own page; elsewhere only what it has
            // made, so a family is not crowded with every bed it might one day make over.
            let pieces: Vec<&'static catalog::Piece> = PIECES
                .iter()
                .filter(|p| p.family == family && shown.is_none_or(|set| p.set == set))
                .filter(|p| {
                    p.set != catalog::Set::Made
                        || shown == Some(catalog::Set::Made)
                        || p.available(&now)
                })
                .collect();
            if pieces.is_empty() {
                continue;
            }
            kicker(ui, family.label());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
                for piece in pieces {
                    let available = piece.available(&now);
                    let carried = matches!(
                        self.arranging.carrying,
                        Some(Carry::New { piece: carrying, .. }) if carrying.id == piece.id
                    );
                    let (response, rect) =
                        tile(ui, egui::Id::new(("piece", piece.id)), size, carried, unit);
                    let (texture, pixels) =
                        self.thumbnail(ctx, format!("piece:{}", piece.id), || {
                            cropped(crate::art::furniture::draw(piece, false).canvas)
                        });
                    let shown = crisp(ui, pixels, size - egui::vec2(5.0, 5.0) * unit, 2.0);
                    let tint = if available {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(70)
                    };
                    ui.painter().image(
                        texture,
                        egui::Rect::from_center_size(rect.center(), shown),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                    let count = house
                        .pieces
                        .iter()
                        .filter(|placed| placed.piece.as_str() == piece.id)
                        .count();
                    if count > 0 {
                        let galley = ui.painter().layout_no_wrap(
                            count.to_string(),
                            egui::FontId::monospace(10.0),
                            ink::forest(dark),
                        );
                        ui.painter().galley(
                            rect.right_bottom()
                                - galley.size()
                                - egui::vec2(2.0 * unit, 1.5 * unit),
                            galley,
                            ink::forest(dark),
                        );
                    }
                    let name = piece.name;
                    let set = piece.set;
                    let when = piece.arrives.when(&now);
                    let response = response.on_hover_ui(|ui| {
                        ui.strong(name);
                        if set == catalog::Set::Made {
                            ui.label(egui::RichText::new("Made at home").small());
                        } else if set != catalog::Set::Home {
                            ui.label(
                                egui::RichText::new(format!("From the {} set", set.name())).small(),
                            );
                        }
                        if let Some(when) = &when {
                            ui.label(egui::RichText::new(when).small());
                        } else if count > 0 {
                            ui.label(egui::RichText::new(format!("{count} in the house")).small());
                        }
                    });
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, available, name)
                    });
                    if available && (response.clicked() || response.drag_started()) {
                        self.arranging.carrying = Some(Carry::New { piece, turn: 0 });
                        self.arranging.dragged = response.drag_started();
                    }
                    if available {
                        response.on_hover_cursor(egui::CursorIcon::Grab);
                    }
                }
            });
            ui.add_space(4.0);
        }
    }

    fn rooms_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        let dark = ui.visuals().dark_mode;
        if self.placing.is_some() {
            self.placing_page(ui);
            return;
        }
        let home = home_of(&self.state, self.keeper).clone();
        if home.rooms.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                for (index, room) in self.house.rooms.iter().enumerate() {
                    let name = catalog::room_name(room.kind.as_ref(), index == 0);
                    ui.selectable_value(&mut self.room_page, index as u8, name);
                }
            });
            ui.add_space(4.0);
        }
        let room = usize::from(self.room_page).min(home.rooms.len() - 1);
        let layout = home.rooms[room].clone();
        let mut floor = None;
        let mut wall = None;
        let now = catalog::Arrived {
            made: &home.projects,
            ..catalog::Arrived::of(&self.household.snapshot, &home)
        };
        kicker(ui, "Floor");
        let size = egui::vec2(34.0 * unit, 22.0 * unit);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
            for finish in &FLOORS {
                let chosen = layout.floor.as_str() == finish.id;
                let (response, rect) =
                    tile(ui, egui::Id::new(("floor", finish.id)), size, chosen, unit);
                let (texture, pixels) = self.thumbnail(ctx, format!("floor:{}", finish.id), || {
                    shell::floor_swatch(finish.id)
                });
                let shown = crisp(ui, pixels, size - egui::vec2(4.0, 4.0) * unit, 4.0);
                let arrived = finish.available(&now);
                ui.painter().image(
                    texture,
                    egui::Rect::from_center_size(rect.center(), shown),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    if arrived {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(70)
                    },
                );
                let name = finish.name;
                let response = response.on_hover_text(match finish.arrives.when(&now) {
                    Some(when) => format!("{name}\n{when}"),
                    None => name.to_owned(),
                });
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, arrived, chosen, name)
                });
                if response.clicked() && arrived {
                    floor = Some(finish.id);
                }
            }
        });
        ui.add_space(4.0);
        kicker(ui, "Walls");
        let size = egui::vec2(22.0 * unit, 34.0 * unit);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
            for finish in &WALLS {
                let chosen = layout.wall.as_str() == finish.id;
                let (response, rect) =
                    tile(ui, egui::Id::new(("wall", finish.id)), size, chosen, unit);
                let (texture, pixels) = self.thumbnail(ctx, format!("wall:{}", finish.id), || {
                    shell::wall_swatch(finish.id)
                });
                let shown = crisp(ui, pixels, size - egui::vec2(4.0, 4.0) * unit, 4.0);
                let arrived = finish.available(&now);
                ui.painter().image(
                    texture,
                    egui::Rect::from_center_size(rect.center(), shown),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    if arrived {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(70)
                    },
                );
                let name = finish.name;
                let response = response.on_hover_text(match finish.arrives.when(&now) {
                    Some(when) => format!("{name}\n{when}"),
                    None => name.to_owned(),
                });
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, arrived, chosen, name)
                });
                if response.clicked() && arrived {
                    wall = Some(finish.id);
                }
            }
        });
        if floor.is_some() || wall.is_some() {
            self.arranging
                .finish(&mut self.state, self.keeper, room as u8, floor, wall);
            self.changed("The room has a new look.");
        }
        if room > 0 && arrange::can_take_away(&home, room as u8) {
            ui.add_space(4.0);
            if ui.small_button("Take this room away").clicked()
                && self
                    .arranging
                    .take_away_room(&mut self.state, self.keeper, room as u8)
            {
                self.changed("The room is gone, and everything in it put away.");
            }
        }
        ui.add_space(8.0);
        let allowed = catalog::rooms_allowed(self.household.snapshot.days_lived);
        if home.rooms.len() < allowed {
            kicker(ui, "Add a room");
            let size = egui::vec2(30.0 * unit, 30.0 * unit);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
                for template in &catalog::ROOMS {
                    let (response, rect) = tile(
                        ui,
                        egui::Id::new(("template", template.id)),
                        size,
                        false,
                        unit,
                    );
                    plan_of(ui.painter(), rect, template.size, unit, dark);
                    let name = template.name;
                    let (w, d) = template.size;
                    let response = response.on_hover_ui(|ui| {
                        ui.strong(name);
                        ui.label(egui::RichText::new(format!("{w} by {d} tiles")).small());
                    });
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                    });
                    if response.clicked() {
                        let places =
                            arrange::room_places(&home, template, &self.household.snapshot);
                        if places.is_empty() {
                            self.say("There is nowhere for that room to go.");
                        } else {
                            self.placing = Some((places, 0));
                        }
                    }
                }
            });
        } else if let Some(days) =
            catalog::days_to_next_room(home.rooms.len(), self.household.snapshot.days_lived)
                .filter(|days| *days > 0)
        {
            let comes = match days {
                1 => "Another room can be built after one more day lived.".to_owned(),
                days => format!("Another room can be built after {days} more days lived."),
            };
            ui.label(
                egui::RichText::new(comes)
                    .small()
                    .italics()
                    .color(ink::muted(dark)),
            );
        }
    }

    /// Choosing where a new room goes: each place it could, shown in the house in turn.
    fn placing_page(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let Some((places, shown)) = &mut self.placing else {
            return;
        };
        kicker(ui, "Where should it go?");
        ui.label(
            egui::RichText::new(format!("{} of {} places", *shown + 1, places.len()))
                .small()
                .color(ink::muted(dark)),
        );
        let mut build = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            if ui
                .button("\u{25c0}")
                .on_hover_text("The place before")
                .clicked()
            {
                *shown = (*shown + places.len() - 1) % places.len();
            }
            if ui
                .button("\u{25b6}")
                .on_hover_text("The next place")
                .clicked()
            {
                *shown = (*shown + 1) % places.len();
            }
        });
        ui.horizontal(|ui| {
            build = ui.button("Build here").clicked();
            cancel = ui.button("Not now").clicked();
        });
        if build {
            let grown = places[*shown].clone();
            self.placing = None;
            let taken_down = self.arranging.build(&mut self.state, self.keeper, &grown);
            self.room_page = (grown.rooms.len() - 1) as u8;
            if let Some(room) = grown.rooms.last().and_then(|room| room.kind.clone()) {
                self.note(HomeMoment::Room { room });
            }
            self.changed(if taken_down.is_empty() {
                "A new room."
            } else {
                "A new room. What hung on the wall it cut down is back in the drawer."
            });
        } else if cancel {
            self.placing = None;
        }
    }

    /// Under every arranging page: taking back, and the keys that save reaching for the mouse.
    fn arranging_foot(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        // A keepsake picked up can be let go from here as well as from its menu.
        let keepsake = match &self.arranging.carrying {
            Some(Carry::Thing(item))
                if self
                    .household
                    .snapshot
                    .item(item)
                    .is_some_and(keepsakes::is_keepsake) =>
            {
                Some(item.clone())
            }
            _ => None,
        };
        let mut let_go = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.arranging.can_undo(), egui::Button::new("Undo"))
                .clicked()
                && self.arranging.undo(&mut self.state)
            {
                self.changed("Undone.");
            }
            if ui
                .add_enabled(self.arranging.can_redo(), egui::Button::new("Redo"))
                .clicked()
                && self.arranging.redo(&mut self.state)
            {
                self.changed("Done again.");
            }
            if keepsake.is_some() {
                let_go = ui
                    .button("Let it go")
                    .on_hover_text("Out of the drawer, to make room for more")
                    .clicked();
            }
        });
        if let (true, Some(item)) = (let_go, keepsake) {
            self.let_keepsake_go(item);
        }
        ui.label(
            egui::RichText::new("Drag to place \u{b7} R turns \u{b7} Delete puts away")
                .small()
                .italics()
                .color(ink::muted(dark)),
        );
        self.help_button(ui);
    }
}

/// A room's plan in small: its floor, and the shape it makes, `size` tiles.
fn plan_of(painter: &egui::Painter, rect: egui::Rect, size: (u8, u8), unit: f32, dark: bool) {
    let cell = (rect.width() - 8.0 * unit) / f32::from(size.0.max(size.1));
    let cell = (cell / unit).floor().max(1.0) * unit;
    let plan = egui::vec2(f32::from(size.0), f32::from(size.1)) * cell;
    let at = egui::Rect::from_center_size(rect.center(), plan);
    painter.rect_filled(at, 0.0, egui::Color32::from_rgb(0xe0, 0xad, 0x72));
    stepped(painter, at.expand(unit), unit, ink::line(dark));
}

/// "the armchair", "the armchair and the shell", "the armchair, the rug and the shell".
pub(super) fn and_list(things: &[String]) -> String {
    match things {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
