//! The Home window: the room, scaled up whole, with the household living in it; a drawer beside
//! it of who is home, or of everything that can go in the room; and the two modes.
//!
//! The window opens in Live Mode, where the owner chooses a resident, then clicks something in
//! the room to see what it could do there; a resident can be carried by dragging it and given a
//! pat with a right-click. Arrange Mode is asked for explicitly, so a chair is never dragged by
//! accident: there everyone waits while furniture and finds are picked up and put down.

use crate::arrange::{self, Arranging, Carry};
use crate::catalog::{self, FLOORS, Family, PIECES, WALLS};
use crate::host::Host;
use crate::house::{At, House};
use crate::household::{Household, Id, Whereabouts};
use crate::keepsakes;
use crate::life::{self, Act, Life};
use crate::placement;
use crate::scene::{Overlay, Scene, Target};
use crate::store::{self, WindowPlace};
use eframe::egui;
use formiga_art::Canvas;
use formiga_home_contract::{
    DisplayId, HomeMoment, HomeState, HouseholdHome, Liked, MementoKind, TravelerId,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;

const NOTICE_SECS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Live,
    Arrange,
}

/// A page of the notes: when arranging, the drawer of things to put in the house; when living
/// in it, who is home, or the household's journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drawer {
    Furniture,
    Finds,
    Room,
    Household,
    Journal,
}

pub struct HomeApp {
    household: Household,
    host: Host,
    state: HomeState,
    keeper: TravelerId,
    /// The visited household's rooms set out as one house, made again whenever they change.
    house: House,
    /// The room the Room page of the drawer is about.
    room_page: u8,
    /// A room being built: every place it could go, as the home it would make, and the one
    /// shown.
    placing: Option<(Vec<HouseholdHome>, usize)>,
    scene: Scene,
    life: Life,
    mode: Mode,
    drawer: Drawer,
    /// The notes page turned to while living in the house.
    live_page: Drawer,
    selected: Option<Id>,
    hovered: Option<Target>,
    menu: Option<Menu>,
    arranging: Arranging,
    carried: Option<Id>,
    notice: Option<(String, f32)>,
    texture: Option<egui::TextureHandle>,
    thumbnails: HashMap<String, egui::TextureHandle>,
    /// The notebook's cover, painted for the window's size.
    chrome: notebook::Chrome,
    /// The sticky note of how to do things, taped over the notes.
    help: bool,
    /// Which set the furniture page shows, or every set.
    set_shown: Option<catalog::Set>,
    /// How close the house is seen.
    zoom: Zoom,
    /// Seconds since the window opened, as egui's input says: one clock for the whole frame.
    clock: f32,
    /// Where the room was drawn last frame.
    room_rect: Option<egui::Rect>,
    last: f32,
    last_recall_check: f32,
    data: Option<PathBuf>,
    place: Option<WindowPlace>,
    left: bool,
    /// When the house was opened, to the second: the journal's lines since are this visit's.
    opened_at_utc: OffsetDateTime,
    /// The house the owner is going over to, if they are going next door.
    next_door: Option<TravelerId>,
    /// A friend asked to move in this visit, who would like to.
    move_in: Option<Id>,
    /// Who Desktop was last told is in the house.
    indoors_said: Option<Vec<Id>>,
    _open: Option<store::Open>,
    /// For review: a picture of the window itself to save, and when, after which it closes; and
    /// how many steps closer than fits to show the house in it.
    snap: Option<(PathBuf, f32, bool)>,
    snap_zoom: i32,
    /// For review: an hour and month to show instead of the clock's.
    fixed_daylight: Option<crate::daylight::Daylight>,
}

/// Now, to the second, as the journal keeps time.
fn now_utc() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(0).unwrap_or(now)
}

/// The visited household's home in `state`.
fn home_of(state: &HomeState, keeper: TravelerId) -> &HouseholdHome {
    state
        .household(keeper)
        .expect("the visited household always has a home")
}

impl HomeApp {
    pub fn new(
        ctx: &egui::Context,
        household: Household,
        host: Host,
        data: Option<PathBuf>,
        open: Option<store::Open>,
    ) -> Self {
        let presentation = household.snapshot.presentation;
        ctx.set_theme(match presentation.theme {
            formiga_travel::Theme::Light => egui::ThemePreference::Light,
            formiga_travel::Theme::Dark => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        });
        notebook::style(ctx);
        ctx.set_zoom_factor(household.text_scale());
        let mut state = host.state().clone();
        arrange::ensure_home(&mut state, &household.snapshot);
        let keeper = household.snapshot.household.keeper;
        let mut household = household;
        keepsakes::stock(&mut household.snapshot, home_of(&state, keeper));
        let house = House::of(&home_of(&state, keeper).rooms);
        let mut scene = Scene::new(&house);
        scene.set_pictures(keepsakes::pictures(home_of(&state, keeper)));
        let mut life = Life::new(&household, &house);
        if host.follows_indoors() {
            let out: Vec<Id> = home_of(&state, keeper)
                .stays_out
                .iter()
                .map(|id| id.0)
                .collect();
            life.start_out(&out);
        }
        Self {
            selected: life.present().first().copied(),
            household,
            host,
            state,
            keeper,
            house,
            room_page: 0,
            placing: None,
            scene,
            life,
            mode: Mode::Live,
            drawer: Drawer::Finds,
            live_page: Drawer::Household,
            hovered: None,
            menu: None,
            arranging: Arranging::default(),
            carried: None,
            notice: None,
            texture: None,
            thumbnails: HashMap::new(),
            chrome: notebook::Chrome::default(),
            help: false,
            set_shown: None,
            zoom: Zoom::default(),
            clock: 0.0,
            room_rect: None,
            last: 0.0,
            last_recall_check: 0.0,
            place: data.as_deref().and_then(WindowPlace::load),
            data,
            left: false,
            opened_at_utc: now_utc(),
            next_door: None,
            move_in: None,
            indoors_said: None,
            _open: open,
            snap: None,
            snap_zoom: 0,
            fixed_daylight: None,
        }
    }

    /// For review only: the house at this hour and month, whatever the clock says.
    pub fn set_daylight(&mut self, daylight: crate::daylight::Daylight) {
        self.fixed_daylight = Some(daylight);
    }

    /// The hour and month at home.
    fn daylight(&self) -> crate::daylight::Daylight {
        self.fixed_daylight
            .unwrap_or_else(crate::daylight::Daylight::now)
    }

    fn now(&self) -> f32 {
        self.clock
    }

    fn say(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), self.now()));
    }

    /// Hand back the homes as they stand, as a crash would otherwise lose them.
    fn keep(&mut self) {
        if let Err(error) = self.host.keep(&self.state) {
            eprintln!("formiga-home: {error:#}");
        }
    }

    /// The owner is leaving: the homes go back to Desktop, and the window remembers where it was.
    fn leave(&mut self) {
        if self.left {
            return;
        }
        self.left = true;
        let lived = self.lived();
        if let Err(error) = self.host.leave(&self.state, &lived) {
            eprintln!("formiga-home: {error:#}");
        }
        if let (Some(data), Some(place)) = (&self.data, self.place) {
            place.save(data);
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        let now = self.now();
        self.menu = None;
        if let Some(id) = self.carried.take() {
            self.life.put_down(&self.house, id, now);
        }
        match mode {
            Mode::Arrange => {
                self.life.pause(&self.household, &self.house, now);
            }
            Mode::Live => {
                self.arranging.carrying = None;
                self.arranging.dragged = false;
                self.placing = None;
                self.life.resume(&self.house, &self.household.snapshot, now);
                self.keep();
            }
        }
        self.mode = mode;
    }

    /// The household's keepsakes have changed: one made, or one let go.
    fn keepsakes_changed(&mut self) {
        let home = home_of(&self.state, self.keeper);
        keepsakes::stock(&mut self.household.snapshot, home);
        self.scene.set_pictures(keepsakes::pictures(home));
        self.thumbnails
            .retain(|key, _| !key.starts_with(&format!("item:{}.", DisplayId::MEMENTO)));
        self.house = House::of(&home.rooms);
    }

    fn name(&self, id: Id) -> String {
        self.household
            .resident(id)
            .map_or_else(|| "Someone".to_owned(), |resident| resident.name.clone())
    }

    /// The room has changed: everyone steps clear of the furniture, and Desktop is handed the
    /// homes as they now stand.
    fn changed(&mut self, notice: &str) {
        if let Some(home) = self.state.household_mut(self.keeper) {
            home.forget_what_is_gone();
        }
        // Taking something back can bring back a keepsake let go, or let one go again.
        let home = home_of(&self.state, self.keeper);
        let kept: Vec<(DisplayId, MementoKind)> = home
            .mementos
            .iter()
            .map(|memento| (memento.id(home.keeper), memento.kind))
            .collect();
        let stocked: Vec<(DisplayId, MementoKind)> = self
            .household
            .snapshot
            .inventory
            .iter()
            .filter_map(|item| match item.source {
                formiga_home_contract::DisplaySource::HomeMemento { memento } => {
                    Some((item.id.clone(), memento))
                }
                _ => None,
            })
            .collect();
        if kept != stocked {
            self.keepsakes_changed();
        }
        self.house = House::of(&home_of(&self.state, self.keeper).rooms);
        self.room_page = self
            .room_page
            .min(self.house.rooms.len().saturating_sub(1) as u8);
        self.life.make_room(&self.house);
        self.keep();
        self.say(notice);
    }

    fn thumbnail(
        &mut self,
        ctx: &egui::Context,
        key: String,
        draw: impl FnOnce() -> Canvas,
    ) -> (egui::TextureId, egui::Vec2) {
        let texture = self.thumbnails.entry(key.clone()).or_insert_with(|| {
            ctx.load_texture(
                key,
                notebook::image_of(&draw()),
                egui::TextureOptions::NEAREST,
            )
        });
        let size = texture.size_vec2();
        (texture.id(), size)
    }

    /// The house as it is drawn just now: the one being built, if a room is being placed.
    fn shown_house(&self) -> House {
        match &self.placing {
            Some((places, shown)) => House::of(&places[*shown].rooms),
            None => self.house_as_seen(),
        }
    }

    fn room_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let now = self.now();
        let house = self.shown_house();
        let page = ui.available_rect_before_wrap();
        let view = crate::iso::View::of(&house);
        let ppp = ctx.pixels_per_point();
        self.zoom.fit = fit_pixels(page, ppp, view.size);
        let rect = self.zoom.place(page, ppp, view.size);
        let visible = rect.intersect(page);
        self.room_rect = Some(rect);
        let response = ui.allocate_rect(visible, egui::Sense::click_and_drag());
        let scale = rect.width() / view.size.0 as f32;
        // While something is carried the pointer is followed wherever it is pressed from, the
        // drawer included; otherwise only while it is over the room.
        let carrying = self.carried.is_some() || self.arranging.dragged;
        let pointer = if carrying {
            ctx.input(|input| input.pointer.latest_pos())
        } else {
            response.hover_pos()
        }
        .filter(|pos| visible.contains(*pos))
        .map(|pos| ((pos.x - rect.min.x) / scale, (pos.y - rect.min.y) / scale));
        let pixel = pointer.map(|(x, y)| (x as i32, y as i32));
        let released = ctx.input(|input| input.pointer.primary_released());

        let snapshot = self.household.snapshot.clone();
        if self.menu.is_none() && self.placing.is_none() {
            self.hovered = pixel.and_then(|point| {
                self.scene
                    .hit(&house, &snapshot, &mut self.life.actors, now, point)
            });
        } else if self.placing.is_some() {
            self.hovered = None;
        }

        let mut overlay = Overlay {
            hovered: self.hovered.clone(),
            selected: self.selected,
            arranging: self.mode == Mode::Arrange,
            lamps_off: self.life.lamps_off().to_vec(),
            daylight: self.daylight(),
            ..Overlay::default()
        };
        match self.mode {
            Mode::Live => self.live_pointer(&response, pointer, released, now),
            Mode::Arrange if self.placing.is_some() => {
                overlay.new_room = self
                    .placing
                    .as_ref()
                    .map(|(places, _)| (places[0].rooms.len() - 1) as u8);
            }
            Mode::Arrange => {
                if self.drawer == Drawer::Room && house.rooms.len() > 1 {
                    overlay.chosen_room = Some(self.room_page);
                }
                self.arrange_pointer(&response, pointer, released);
                let (ghost, door) = self.ghost(pointer);
                overlay.ghost = ghost;
                overlay.door = door;
                overlay.lifted = match &self.arranging.carrying {
                    Some(Carry::Piece { name, .. }) => Some(Target::Piece(*name)),
                    Some(Carry::Thing(id)) => Some(Target::Shown(id.clone())),
                    _ => None,
                };
                if self.arranging.carrying.is_some() {
                    overlay.hovered = None;
                }
            }
        }
        if self.hovered.is_some() || self.arranging.carrying.is_some() {
            ctx.set_cursor_icon(
                if self.carried.is_some() || self.arranging.carrying.is_some() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::PointingHand
                },
            );
        }

        let house = self.shown_house();
        let canvas = self
            .scene
            .compose(&house, &snapshot, &mut self.life.actors, now, &overlay);
        let texture = notebook::show_in(&mut self.texture, ctx, "room", &canvas);
        self.zoom_input(&response, ctx, page);
        let painter = ui.painter_at(page);
        painter.image(
            texture,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        // What is pointed at, in a tag over it: who, or what and whose favourite it is.
        if let Some((name, (x, y))) = self.tag(&house, now)
            && self.mode == Mode::Live
        {
            // A slip of card with the notebook's stepped edge, written in its ink.
            let dark = ui.visuals().dark_mode;
            let unit = notebook::unit(ctx.pixels_per_point());
            let at = rect.min + egui::vec2(x, y) * scale;
            let font = egui::FontId::proportional(13.0);
            let galley = painter.layout_no_wrap(name, font, notebook::ink::page(dark));
            let tag = egui::Rect::from_center_size(at, galley.size() + egui::vec2(12.0, 6.0));
            let tag = tag.translate(egui::vec2(
                (visible.min.x + 2.0 - tag.min.x).max(0.0)
                    + (visible.max.x - 2.0 - tag.max.x).min(0.0),
                (visible.min.y + 2.0 - tag.min.y).max(0.0),
            ));
            painter.rect_filled(
                tag.translate(egui::vec2(unit, unit)),
                0.0,
                egui::Color32::from_black_alpha(50),
            );
            painter.rect_filled(tag.shrink(unit), 0.0, notebook::ink::card(dark));
            pages::stepped(&painter, tag, unit, notebook::ink::line(dark));
            painter.galley(
                tag.center() - galley.size() / 2.0,
                galley,
                notebook::ink::page(dark),
            );
        }
    }

    /// The house as it looks just now: with any find being worn lifted out of it.
    fn house_as_seen(&self) -> House {
        let mut house = self.house.clone();
        let worn = self.life.worn();
        house.shown.retain(|shown| !worn.contains(&shown.item));
        house
    }

    /// Whose favourite something in the house is: "Mochi's favourite".
    fn favourite_of(&self, liked: &Liked) -> Option<String> {
        let home = self.state.household(self.keeper)?;
        let names: Vec<String> = self
            .household
            .residents
            .iter()
            .filter(|resident| {
                life::favourites(&home.likings, &self.house, resident.id)
                    .iter()
                    .any(|(_, thing)| thing == liked)
            })
            .map(|resident| resident.name.clone())
            .collect();
        match names.as_slice() {
            [] => None,
            [one] => Some(format!("{one}'s favourite")),
            [rest @ .., last] => Some(format!("{} and {last}'s favourite", rest.join(", "))),
        }
    }

    /// The tag for whatever is pointed at, and where over it on the scene.
    fn tag(&mut self, house: &House, now: f32) -> Option<(String, (f32, f32))> {
        let view = self.scene.view;
        match self.hovered.clone()? {
            Target::Resident(id) => {
                let actor = self.life.actors.iter_mut().find(|actor| actor.id == id)?;
                let (l, t, r, _) = actor.bounds(&view, now);
                let mut name = self.name(id);
                if self.life.staying(id) {
                    name.push_str(", staying over");
                } else if self.household.is_visitor(id) {
                    name.push_str(", visiting");
                }
                Some((name, ((l + r) as f32 / 2.0, t as f32 - 9.0)))
            }
            Target::Piece(uid) => {
                let placed = house.piece(uid)?;
                let piece = catalog::piece(&placed.piece)?;
                let (cx, cy) = placement::footprint(placed).centre();
                let (sx, sy) = view.screen(cx, cy);
                let mut name = piece.name.to_owned();
                if let Some(whose) = house.liked(uid).and_then(|liked| self.favourite_of(&liked)) {
                    name = format!("{name} \u{b7} {whose}");
                }
                Some((name, (sx, sy - piece.height as f32 - 8.0)))
            }
            Target::Shown(item) => {
                let at = house.shown.iter().find(|shown| shown.item == item)?.at;
                let (x, y) = self.scene.spot_anchor(house, at)?;
                let mut name = self.household.snapshot.item(&item)?.name.clone();
                if let Some(whose) = self.favourite_of(&Liked::Shown { item }) {
                    name = format!("{name} \u{b7} {whose}");
                }
                Some((name, (x as f32, y as f32 - 16.0)))
            }
            Target::Floor(..) => None,
        }
    }

    /// One frame of the window, from whatever input egui has gathered for it.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.clock = ctx.input(|input| input.time) as f32;
        let now = self.now();
        let dt = (now - self.last).clamp(0.0, 0.25);
        self.last = now;
        let (close, outer, inner) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.close_requested(),
                viewport.outer_rect,
                viewport.inner_rect,
            )
        });
        if let (Some(outer), Some(inner)) = (outer, inner) {
            self.place = Some(WindowPlace {
                x: outer.min.x,
                y: outer.min.y,
                width: inner.width(),
                height: inner.height(),
            });
        }
        if close {
            self.leave();
        }
        if now - self.last_recall_check > 1.0 {
            self.last_recall_check = now;
            if self.host.recalled() {
                // Desktop has the household back already: close, and write nothing.
                self.left = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.keys(&ctx);
        if self.mode == Mode::Live {
            self.life.set_dark(self.daylight().dark().0);
            let home = home_of(&self.state, self.keeper);
            self.life.tick(
                &self.household,
                &self.house,
                &self.household.snapshot,
                &home.likings,
                now,
                dt,
            );
            self.events();
        }
        self.say_who_is_indoors();
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, since)| now - since > NOTICE_SECS)
        {
            self.notice = None;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let layout = self.notebook(ui);
                let page = layout.left.shrink(6.0 * layout.unit);
                ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
                    self.room_view(ui, &ctx);
                });
                self.zoom_controls(ui, page, layout.unit);
                self.cells(ui, page, layout.unit);
                ui.scope_builder(egui::UiBuilder::new().max_rect(layout.notes), |ui| {
                    self.notes(ui, &ctx, layout.unit);
                });
                self.help_note(ui, &ctx, layout.notes, layout.unit);
                self.status_line(ui, &layout);
            });
        self.menu(&ctx);
        self.take_snap(&ctx);
        ctx.request_repaint_after(Duration::from_millis(33));
    }

    /// The line along the foot of the cover: what whoever is chosen is doing, or what has just
    /// happened; and, for a rehearsal, which one.
    fn status_line(&mut self, ui: &mut egui::Ui, layout: &notebook::Layout) {
        let dark = layout.dark;
        let painter = ui.painter();
        let (text, italic) = match &self.notice {
            Some((notice, _)) => (notice.clone(), true),
            None => match self.selected {
                Some(id) => (
                    self.life
                        .doing(&self.household, &self.house, &self.household.snapshot, id),
                    false,
                ),
                None => (String::new(), false),
            },
        };
        let mut format = egui::TextFormat {
            font_id: egui::FontId::proportional(13.0),
            color: notebook::ink::cover(dark),
            italics: italic,
            ..Default::default()
        };
        let mut job = egui::text::LayoutJob::default();
        job.append(&text, 0.0, format.clone());
        let galley = painter.layout_job(job);
        let at = egui::pos2(
            layout.status.min.x,
            layout.status.center().y - galley.size().y / 2.0,
        );
        painter.galley(at, galley, notebook::ink::cover(dark));
        if let Some(label) = self.host.rehearsal_label() {
            format.font_id = egui::FontId::proportional(11.0);
            format.color = notebook::ink::deboss(dark);
            format.italics = false;
            let mut job = egui::text::LayoutJob::default();
            job.append(label, 0.0, format);
            let galley = painter.layout_job(job);
            let at = egui::pos2(
                layout.status.max.x - galley.size().x,
                layout.status.center().y - galley.size().y / 2.0,
            );
            painter.galley(at, galley, notebook::ink::deboss(dark));
        }
    }
}

/// Where in this house a thing is shown, as the drawer says it: "on the open shelf", or in a
/// house of rooms, "on the open shelf in the gallery".
fn where_here(house: &House, item: &DisplayId) -> String {
    let Some(shown) = house.shown.iter().find(|shown| &shown.item == item) else {
        return "put away".to_owned();
    };
    let what = match shown.at {
        At::On { piece, .. } => house
            .piece(piece)
            .and_then(|placed| catalog::piece(&placed.piece))
            .map_or_else(
                || "on a shelf".to_owned(),
                |piece| format!("on the {}", piece.name.to_lowercase()),
            ),
        At::Wall { .. } => "on the wall".to_owned(),
        At::Floor { .. } => "on the floor".to_owned(),
    };
    let room = house.spot(shown.at).map(|(room, _)| room);
    match room.and_then(|room| house.room(room).map(|at| (room, at))) {
        Some((index, at)) if house.rooms.len() > 1 => format!(
            "{what} in the {}",
            catalog::room_name(at.kind.as_ref(), index == 0).to_lowercase()
        ),
        _ => what,
    }
}

impl eframe::App for HomeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    /// A window opened only to have its picture taken answers to nobody: whatever the pointer or
    /// the keys do on it is let go, so a review never changes anything.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.snap.is_some() {
            raw_input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        }
    }

    /// The leather, under everything, until the cover is painted over it.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.357, 0.227, 0.153, 1.0]
    }
}

impl Drop for HomeApp {
    fn drop(&mut self) {
        self.leave();
    }
}

mod arranging;
mod cells;
mod events;
mod keys;
mod living;
mod menu;
mod notebook;
mod pages;
mod photo;
mod visits;
mod zoom;

use menu::{Entry, Menu};
use zoom::{Zoom, fit_pixels};

pub use notebook::frameless;

#[cfg(test)]
mod tests;
