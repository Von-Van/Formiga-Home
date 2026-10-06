//! What is written on the notes page. Living in the house, it is who is home: a small portrait
//! and a name each, and only for whoever is chosen a little more, what it is doing, what it has
//! come to like, and what it has been asked. Arranging, it is the found things, the furniture
//! and the rooms, each a page of small pictures rather than a list, with what each one is kept
//! for when it is pointed at.

use super::notebook::{LABEL, ink, kicker, label_job};
use super::*;
use crate::art::{displays, shell};
use formiga_art::{BodyClip, CreatureRenderer, EyelidPose, FaceRenderState, GazeDirection};
use formiga_core::ActionKind;
use formiga_home_contract::DisplayItem;

/// How big a picture can be drawn inside `fit` points, whole screen pixels to each of its own,
/// and never more than `most` of them.
fn crisp(ui: &egui::Ui, pixels: egui::Vec2, fit: egui::Vec2, most: f32) -> egui::Vec2 {
    let ppp = ui.ctx().pixels_per_point();
    let across = (fit.x * ppp / pixels.x.max(1.0)).min(fit.y * ppp / pixels.y.max(1.0));
    pixels * across.floor().clamp(1.0, most) / ppp
}

/// A picture cut down to what is drawn in it, so it is shown as big as its tile allows.
fn cropped(canvas: Canvas) -> Canvas {
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
fn tile(
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
    /// The notes page, as it is for the mode and the page turned to.
    pub(super) fn notes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unit: f32) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.mode {
                Mode::Live if self.live_page == Drawer::Journal => self.journal_page(ui, unit),
                Mode::Live => self.household_page(ui, ctx, unit),
                Mode::Arrange => {
                    match self.drawer {
                        Drawer::Furniture => self.furniture_page(ui, ctx, unit),
                        Drawer::Room => self.rooms_page(ui, ctx, unit),
                        _ => self.finds_page(ui, ctx, unit),
                    }
                    ui.add_space(10.0);
                    self.arranging_foot(ui);
                }
            });
    }

    /// A resident's face for the notes: its idle frame, cut down to its head and shoulders.
    fn portrait(&mut self, ctx: &egui::Context, id: Id) -> Option<(egui::TextureId, egui::Vec2)> {
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
            ui.label(
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
                    ui.add_space(3.0 * unit);
                }
                ui.label(
                    egui::RichText::new(crate::journal::heading(day, today))
                        .small()
                        .color(ink::forest(dark)),
                );
                last = Some(day);
            }
            ui.label(crate::journal::line(
                &entry.moment,
                &self.household.snapshot,
            ));
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
        ui.horizontal_wrapped(|ui| {
            // A house's name moves to the next line whole rather than breaking.
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.x = 10.0;
            for (keeper, name) in &houses {
                let link = ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(name.as_str())
                                .underline()
                                .color(ink::forest(dark)),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
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
        let residents: Vec<Id> = self.household.residents.iter().map(|r| r.id).collect();
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
            ui.add_space(2.0);
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
        ui.label(
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
        ui.add_space(4.0);
        let word = if self.help {
            "Put the note away"
        } else {
            "How to\u{2026}"
        };
        let response = ui.add(
            egui::Label::new(
                egui::RichText::new(word)
                    .small()
                    .underline()
                    .color(ink::forest(dark)),
            )
            .sense(egui::Sense::click()),
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
        let text = match self.mode {
            Mode::Live => {
                "Click someone to choose them, then click a seat, a toy, a find or someone else \
                 to see what they could do there. Click the floor to send them over.\n\n\
                 Drag someone to carry them, and right-click for a pat. Up to three things can \
                 be asked at once; when nothing is, everyone does as they please.\n\n\
                 P saves a picture of the house."
            }
            Mode::Arrange => {
                "Drag something from these pages, or click it, to pick it up, and click again to \
                 put it down. A doorway slides along its wall.\n\n\
                 R or a right-click turns a piece, Delete puts it away, and \u{2318}Z undoes.\n\n\
                 Everyone waits while the house is arranged."
            }
        };
        let width = notes.width().min(260.0);
        let galley = ui.painter().layout(
            text.to_owned(),
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
        let size = egui::vec2(ui.available_width(), 22.0 * unit);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
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
            let frame = egui::Rect::from_min_size(rect.min, egui::vec2(22.0 * unit, 22.0 * unit));
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
        let y = rect.center().y - galley.size().y / 2.0;
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
            let y = rect.center().y - galley.size().y / 2.0 + 1.0;
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
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                egui::RichText::new(phrase)
                    .size(12.0)
                    .italics()
                    .color(ink::muted(dark)),
            );
            if !doing.is_empty() {
                ui.label(egui::RichText::new(self.in_which_room(id, doing)).size(12.0));
            }
            if !liked.is_empty() {
                ui.label(
                    egui::RichText::new(format!("Likes {}", and_list(liked)))
                        .size(12.0)
                        .color(ink::muted(dark)),
                );
            }
            let queue = self.life.queue(id);
            if !queue.is_empty() {
                ui.add_space(2.0);
                ui.label(label_job("Asked", LABEL, ink::forest(dark)));
                let mut cancel = None;
                for (position, act) in queue.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("\u{2715}")
                            .on_hover_text("Take this back")
                            .clicked()
                        {
                            cancel = Some(position);
                        }
                        ui.label(
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
            Some(Tapped::LetGo(item)) => {
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
            None => {}
        }
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
        let snapshot = &self.household.snapshot;
        let (days, things) = (
            snapshot.days_lived,
            crate::keepsakes::colony_things(snapshot).count(),
        );
        let house = self.house.clone();
        let size = egui::vec2(30.0 * unit, 30.0 * unit);
        // The sets, to look at one at a time.
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.set_shown, None, "All");
            for set in catalog::Set::ALL {
                ui.selectable_value(&mut self.set_shown, Some(set), set.name())
                    .on_hover_text(if set.arrives().come(days, things) {
                        "Here to use"
                    } else {
                        "Arrives in time"
                    });
            }
        });
        ui.add_space(4.0);
        let shown = self.set_shown;
        for family in Family::ALL {
            let pieces: Vec<&'static catalog::Piece> = PIECES
                .iter()
                .filter(|p| p.family == family && shown.is_none_or(|set| p.set == set))
                .collect();
            if pieces.is_empty() {
                continue;
            }
            kicker(ui, family.label());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0 * unit, 2.0 * unit);
                for piece in pieces {
                    let available = piece.available(days, things);
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
                    let response = response.on_hover_ui(|ui| {
                        ui.strong(name);
                        if set != catalog::Set::Home {
                            ui.label(
                                egui::RichText::new(format!("From the {} set", set.name())).small(),
                            );
                        }
                        if !available {
                            ui.label(egui::RichText::new("Arrives in time").small());
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
        let snapshot = &self.household.snapshot;
        let (days, things) = (
            snapshot.days_lived,
            crate::keepsakes::colony_things(snapshot).count(),
        );
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
                let arrived = finish.set.arrives().come(days, things);
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
                let response = response.on_hover_text(if arrived {
                    name.to_owned()
                } else {
                    format!("{name}: arrives in time")
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
                let arrived = finish.set.arrives().come(days, things);
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
                let response = response.on_hover_text(if arrived {
                    name.to_owned()
                } else {
                    format!("{name}: arrives in time")
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
        } else if home.rooms.len() < formiga_home_contract::limits::MAX_ROOMS {
            ui.label(
                egui::RichText::new("Another room comes with time.")
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
        });
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
fn and_list(things: &[String]) -> String {
    match things {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
