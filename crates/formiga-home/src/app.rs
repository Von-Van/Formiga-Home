//! The Home window: the room, scaled up whole, with the household living in it; a drawer beside
//! it of who is home, or of everything that can go in the room; and the two modes.
//!
//! The window opens in Live Mode, where the owner chooses a resident, then clicks something in
//! the room to see what it could do there; a resident can be carried by dragging it and given a
//! pat with a right-click. Arrange Mode is asked for explicitly, so a chair is never dragged by
//! accident: there everyone waits while furniture and finds are picked up and put down.

use crate::arrange::{self, Arranging, Carry, Landing};
use crate::catalog::{self, FLOORS, Family, PIECES, WALLS};
use crate::host::Host;
use crate::house::{At, House};
use crate::household::{Household, Id, Whereabouts};
use crate::keepsakes;
use crate::life::{self, Act, Asked, Event, Life, QUEUE_LIMIT, Used, choices};
use crate::room::{self, Place, Showing};
use crate::scene::{Ghost, Overlay, Scene, Target};
use crate::session::Lived;
use crate::store::{self, WindowPlace};
use eframe::egui;
use formiga_art::Canvas;
use formiga_home_contract::{
    DisplayId, FavouriteKind, HomeMoment, HomeState, HouseholdHome, Liked, MementoKind, TravelerId,
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

/// What can be chosen from the menu over the room.
#[derive(Clone, Debug, PartialEq)]
enum Entry {
    Ask(Act),
    Pet,
    Choose(Id),
    /// Ask a visitor to stay over.
    StayOver(Id),
    /// Ask a visitor to come and live here.
    MoveIn(Id),
}

/// How the house is shown on its page: as big as fits, or zoomed in by whole pixels and moved
/// about, for a house that has grown too big to see closely all at once.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Zoom {
    /// Screen pixels to each of the picture's; none for as many as fit.
    pixels: Option<f32>,
    /// As many as fit, on the page as it was last drawn.
    fit: f32,
    /// How far the picture is moved from the middle of its page, in points.
    pan: egui::Vec2,
    /// Being moved about by a drag across the floor.
    panning: bool,
    /// A pinch, or a scroll with Ctrl or ⌘, adding up until it makes a step.
    pinch: f32,
}

/// The closest the house can be seen: this many screen pixels to each of its own.
const CLOSEST: f32 = 12.0;

impl Zoom {
    /// Where the picture goes on its page, `size` pixels of it: centred where it fits, and moved
    /// no further than keeps the page covered where it does not.
    fn place(&mut self, page: egui::Rect, pixels_per_point: f32, size: (u32, u32)) -> egui::Rect {
        let pixels = self.pixels.unwrap_or(self.fit);
        let shown = egui::vec2(size.0 as f32, size.1 as f32) * pixels / pixels_per_point;
        let room = ((shown - page.size()) / 2.0).max(egui::Vec2::ZERO);
        self.pan = self.pan.clamp(-room, room);
        egui::Rect::from_center_size(page.center() + self.pan, shown)
    }

    fn closer_than_fits(&self) -> bool {
        self.pixels.is_some_and(|pixels| pixels > self.fit)
    }

    /// A whole pixel closer (`by` 1) or further (-1), keeping what is at `anchor`, a point from
    /// the page's middle, where it is. Never further than fits.
    fn step(&mut self, by: i32, anchor: egui::Vec2) {
        let old = self.pixels.unwrap_or(self.fit).max(0.1);
        let new = old.floor() + by as f32;
        self.pixels = if new <= self.fit.floor() {
            None
        } else {
            Some(new.min(CLOSEST))
        };
        let new = self.pixels.unwrap_or(self.fit).max(0.1);
        self.pan = anchor - (anchor - self.pan) * (new / old);
        if self.pixels.is_none() {
            self.pan = egui::Vec2::ZERO;
        }
    }

    fn fit(&mut self) {
        self.pixels = None;
        self.pan = egui::Vec2::ZERO;
    }
}

struct Menu {
    at: egui::Pos2,
    title: String,
    entries: Vec<(Entry, String)>,
    opened: f32,
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
        ctx.set_zoom_factor(f32::from(presentation.text_scale_percent.clamp(100, 150)) / 100.0);
        let mut state = host.state().clone();
        arrange::ensure_home(&mut state, &household.snapshot);
        let keeper = household.snapshot.household.keeper;
        let mut household = household;
        keepsakes::stock(&mut household.snapshot, home_of(&state, keeper));
        let house = House::of(&home_of(&state, keeper).rooms);
        let mut scene = Scene::new(&house);
        scene.set_pictures(keepsakes::pictures(home_of(&state, keeper)));
        let life = Life::new(&household, &house);
        Self {
            selected: household.residents.first().map(|resident| resident.id),
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

    /// For review only: open on `page` of the arranging notes, or living in the house if none,
    /// and after `at` seconds save a picture of the window to `path` and close.
    pub fn snap(&mut self, path: PathBuf, at: f32, page: Option<&str>, zoom: i32) {
        self.snap = Some((path, at, false));
        self.snap_zoom = zoom;
        let page = match page {
            Some("finds") => Some(Drawer::Finds),
            Some("furniture") => Some(Drawer::Furniture),
            Some("rooms") => Some(Drawer::Room),
            Some("journal") => {
                self.live_page = Drawer::Journal;
                None
            }
            _ => None,
        };
        if let Some(page) = page {
            self.drawer = page;
            self.set_mode(Mode::Arrange);
        }
    }

    /// Ask for the picture when it is time, and save it when it comes.
    fn take_snap(&mut self, ctx: &egui::Context) {
        let Some((path, at, asked)) = self.snap.clone() else {
            return;
        };
        // Once the page has been laid out once, so the zoom knows what fits.
        if self.snap_zoom > 0 && self.zoom.fit > 0.0 {
            for _ in 0..self.snap_zoom {
                self.zoom.step(1, egui::Vec2::ZERO);
            }
            self.snap_zoom = 0;
        }
        if !asked && self.now() >= at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.snap = Some((path.clone(), at, true));
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let mut canvas = Canvas::new(width as u32, height as u32);
            for (index, pixel) in image.pixels.iter().enumerate() {
                let [r, g, b, a] = pixel.to_srgba_unmultiplied();
                canvas.set(
                    (index % width) as i32,
                    (index / width) as i32,
                    formiga_art::Rgba::new(r, g, b, a),
                );
            }
            if let Err(error) = crate::write_png(&path, &canvas, 1) {
                eprintln!("formiga-home: {error:#}");
            }
            self.snap = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
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

    fn ask(&mut self, id: Id, act: Act) {
        let now = self.now();
        let label = act.label(&self.household, &self.house, &self.household.snapshot);
        let name = self.name(id);
        match self.life.ask(id, act, now) {
            Asked::Queued => self.say(format!("{name}: {}", label.to_lowercase())),
            Asked::Full => self.say(format!(
                "{name} has {QUEUE_LIMIT} things to do already. Take one back in the drawer, or wait."
            )),
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        let (escape, turn, away, undo, redo, photo) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
                command && !input.modifiers.shift && input.key_pressed(egui::Key::Z),
                command && input.modifiers.shift && input.key_pressed(egui::Key::Z),
                input.key_pressed(egui::Key::P),
            )
        });
        // Closer and further, without ⌘ or Ctrl, which zoom the whole window instead.
        let (closer, further, fit) = ctx.input(|input| {
            let plain = !input.modifiers.command;
            (
                plain
                    && (input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)),
                plain && input.key_pressed(egui::Key::Minus),
                plain && input.key_pressed(egui::Key::Num0),
            )
        });
        // The notebook by the keys: the modes, its pages, whoever is chosen, and its note.
        let (live, arrange, back, on, next, previous, note) = ctx.input(|input| {
            let plain = !input.modifiers.command && !input.modifiers.alt;
            let shift = input.modifiers.shift;
            (
                plain && input.key_pressed(egui::Key::L),
                plain && input.key_pressed(egui::Key::A),
                plain && input.key_pressed(egui::Key::OpenBracket),
                plain && input.key_pressed(egui::Key::CloseBracket),
                plain && !shift && input.key_pressed(egui::Key::N),
                plain && shift && input.key_pressed(egui::Key::N),
                plain
                    && (input.key_pressed(egui::Key::H)
                        || input.key_pressed(egui::Key::Questionmark)),
            )
        });
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if live {
            self.set_mode(Mode::Live);
        }
        if arrange {
            self.set_mode(Mode::Arrange);
        }
        if back || on {
            self.turn_page(if on { 1 } else { -1 });
        }
        if (next || previous) && self.mode == Mode::Live {
            self.choose_along(if next { 1 } else { -1 });
        }
        if note {
            self.help = !self.help;
        }
        if closer {
            self.zoom.step(1, egui::Vec2::ZERO);
        }
        if further {
            self.zoom.step(-1, egui::Vec2::ZERO);
        }
        if fit {
            self.zoom.fit();
        }
        if photo {
            self.save_photo();
        }
        if escape {
            if self.placing.take().is_none()
                && self.menu.take().is_none()
                && self.arranging.carrying.take().is_none()
            {
                self.selected = None;
            }
            self.arranging.dragged = false;
        }
        if self.mode != Mode::Arrange {
            return;
        }
        if turn {
            self.arranging.turn();
        }
        if away
            && self
                .arranging
                .put_away(&mut self.state, self.keeper, &self.house)
        {
            self.changed("Put away.");
        }
        if undo && self.arranging.undo(&mut self.state) {
            self.changed("Undone.");
        }
        if redo && self.arranging.redo(&mut self.state) {
            self.changed("Done again.");
        }
    }

    /// The next page of the notes, or the one before, in the mode the house is in.
    fn turn_page(&mut self, by: i32) {
        let pages = notebook::pages(self.mode);
        let at = pages
            .iter()
            .position(|(page, _)| *page == self.page())
            .unwrap_or(0) as i32;
        let (page, _) = pages[(at + by).rem_euclid(pages.len() as i32) as usize];
        match self.mode {
            Mode::Live => self.live_page = page,
            Mode::Arrange => {
                self.drawer = page;
                self.placing = None;
            }
        }
    }

    /// Choose the next one in the house, or the one before: residents, then visitors.
    fn choose_along(&mut self, by: i32) {
        let present = self.life.present();
        let order: Vec<Id> = self
            .household
            .everyone()
            .map(|resident| resident.id)
            .filter(|id| present.contains(id))
            .collect();
        if order.is_empty() {
            return;
        }
        let at = self
            .selected
            .and_then(|id| order.iter().position(|other| *other == id))
            .map_or(if by > 0 { -1 } else { 0 }, |at| at as i32);
        self.selected = Some(order[(at + by).rem_euclid(order.len() as i32) as usize]);
        self.menu = None;
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
            let canvas = draw();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas.width() as usize, canvas.height() as usize],
                &canvas.rgba_bytes(),
            );
            ctx.load_texture(key, image, egui::TextureOptions::NEAREST)
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
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [canvas.width() as usize, canvas.height() as usize],
            &canvas.rgba_bytes(),
        );
        let texture = match &mut self.texture {
            Some(texture) => {
                texture.set(image, egui::TextureOptions::NEAREST);
                texture.id()
            }
            None => {
                let texture = ctx.load_texture("room", image, egui::TextureOptions::NEAREST);
                let id = texture.id();
                self.texture = Some(texture);
                id
            }
        };
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

    /// Moving about a house seen close, and coming closer or going further: a scroll moves it,
    /// a drag across the floor moves it, and a pinch, or a scroll with Ctrl or ⌘, zooms.
    fn zoom_input(&mut self, response: &egui::Response, ctx: &egui::Context, page: egui::Rect) {
        let (scroll, pinch, moved, down, at) = ctx.input(|input| {
            (
                input.smooth_scroll_delta,
                input.zoom_delta(),
                input.pointer.delta(),
                input.pointer.primary_down(),
                input.pointer.latest_pos(),
            )
        });
        if self.zoom.panning {
            if down {
                self.zoom.pan += moved;
            } else {
                self.zoom.panning = false;
            }
        }
        if !response.contains_pointer() {
            return;
        }
        let anchor = at.map_or(egui::Vec2::ZERO, |at| at - page.center());
        if (pinch - 1.0).abs() > f32::EPSILON {
            self.zoom.pinch += pinch.ln();
            if self.zoom.pinch > 0.2 {
                self.zoom.step(1, anchor);
                self.zoom.pinch = 0.0;
            } else if self.zoom.pinch < -0.2 {
                self.zoom.step(-1, anchor);
                self.zoom.pinch = 0.0;
            }
        } else if scroll != egui::Vec2::ZERO && self.zoom.closer_than_fits() {
            self.zoom.pan += scroll;
        }
    }

    /// Closer, as big as fits, and further: three small buttons on a card at the page's corner.
    fn zoom_controls(&mut self, ui: &mut egui::Ui, page: egui::Rect, unit: f32) {
        let dark = ui.visuals().dark_mode;
        let cell = egui::vec2(15.0 * unit, 12.0 * unit);
        let card = egui::Rect::from_min_size(
            page.max - egui::vec2(cell.x * 3.0 + 3.0 * unit, cell.y + 3.0 * unit),
            egui::vec2(cell.x * 3.0, cell.y),
        );
        let painter = ui.painter().clone();
        painter.rect_filled(card.shrink(unit), 0.0, notebook::ink::card(dark));
        pages::stepped(
            &painter,
            card,
            unit,
            notebook::ink::line(dark).gamma_multiply(0.7),
        );
        let fitted = self.zoom.pixels.is_none();
        let cells = [
            ("out", "\u{2212}", "Further away (\u{2212})"),
            ("fit", "Fit", "The whole house (0)"),
            ("in", "+", "Closer (+)"),
        ];
        for (index, (name, label, hint)) in cells.into_iter().enumerate() {
            let rect =
                egui::Rect::from_min_size(card.min + egui::vec2(cell.x * index as f32, 0.0), cell);
            let response = ui
                .interact(rect, egui::Id::new(("zoom", name)), egui::Sense::click())
                .on_hover_text(hint);
            let lit = response.hovered() || (name == "fit" && fitted);
            if lit {
                painter.rect_filled(
                    rect.shrink(unit),
                    0.0,
                    notebook::ink::mint(dark).gamma_multiply(if response.hovered() {
                        1.0
                    } else {
                        0.6
                    }),
                );
            }
            let galley = painter.layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(if name == "fit" { 11.0 } else { 14.0 }),
                notebook::ink::page(dark),
            );
            painter.galley(
                rect.center() - galley.size() / 2.0,
                galley,
                notebook::ink::page(dark),
            );
            if response.clicked() {
                match name {
                    "in" => self.zoom.step(1, egui::Vec2::ZERO),
                    "out" => self.zoom.step(-1, egui::Vec2::ZERO),
                    _ => self.zoom.fit(),
                }
            }
        }
    }

    /// The house as it looks just now: with any find being worn lifted out of it.
    fn house_as_seen(&self) -> House {
        let mut house = self.house.clone();
        let worn = self.life.worn();
        house.shown.retain(|shown| !worn.contains(&shown.item));
        house
    }

    /// What has happened in the house since the last frame.
    fn events(&mut self) {
        for event in self.life.take_events() {
            match event {
                Event::Arrived(id) => {
                    let notice = match self.household.friend_of(id) {
                        Some(friend) => {
                            format!("{} has come over to see {}.", self.name(id), friend.name)
                        }
                        None => format!("{} has come over.", self.name(id)),
                    };
                    self.say(notice);
                    self.note(HomeMoment::Visit {
                        visitor: TravelerId(id),
                    });
                }
                Event::Left(id, gift) => {
                    let left = gift.and_then(|kind| {
                        self.make_keepsake(kind, Some(id), Vec::new()).map(|_| kind)
                    });
                    match left {
                        Some(kind) => self.say(format!(
                            "{} has gone home, and left {} for the drawer.",
                            self.name(id),
                            keepsakes::a(kind)
                        )),
                        None => self.say(format!("{} has gone home.", self.name(id))),
                    }
                    if self.selected == Some(id) {
                        self.selected = None;
                    }
                }
                Event::StayingOver(id) => {
                    self.say(format!("{} is staying over.", self.name(id)));
                    self.note(HomeMoment::StayedOver {
                        visitor: TravelerId(id),
                    });
                }
                Event::Drew(by, of) => {
                    if self
                        .make_keepsake(MementoKind::Drawing, Some(by), vec![of])
                        .is_some()
                    {
                        let whom = if of == by {
                            "itself".to_owned()
                        } else {
                            self.name(of)
                        };
                        self.say(format!(
                            "{} has drawn {whom}. The drawing is in the drawer.",
                            self.name(by)
                        ));
                    }
                }
                Event::Used(id, used) => {
                    let liked = match used {
                        Used::Piece(name) => self.house.liked(name),
                        Used::Shown(item) => Some(Liked::Shown { item }),
                    };
                    let Some(liked) = liked else { continue };
                    let was = self.favourites(id).iter().any(|(_, thing)| *thing == liked);
                    if let Some(home) = self.state.household_mut(self.keeper) {
                        home.note_use(TravelerId(id), liked.clone());
                    }
                    // A favourite just now come to is worth a line in the journal.
                    let now = self
                        .favourites(id)
                        .into_iter()
                        .find(|(_, thing)| *thing == liked)
                        .map(|(kind, _)| kind);
                    if let (Some(kind), false) = (now, was) {
                        self.note(HomeMoment::Favourite {
                            resident: TravelerId(id),
                            thing: match kind {
                                life::Kind::Seat => FavouriteKind::Seat,
                                life::Kind::Bed => FavouriteKind::Bed,
                                life::Kind::Toy => FavouriteKind::Toy,
                                life::Kind::Find => FavouriteKind::Find,
                            },
                        });
                    }
                }
            }
        }
    }

    /// A resident's favourites in the house, as they now stand.
    fn favourites(&self, id: Id) -> Vec<(life::Kind, Liked)> {
        life::favourites(&home_of(&self.state, self.keeper).likings, &self.house, id)
    }

    /// A line in the household's journal, as of now.
    fn note(&mut self, moment: HomeMoment) {
        if let Some(home) = self.state.household_mut(self.keeper) {
            home.note(now_utc(), moment);
        }
    }

    /// What was lived in the house while it was open, for Desktop: who spent time together and
    /// how, and the few moments most worth a line in its journal — a keepsake first, then a new
    /// room, a new favourite, a friend come over — in the order they happened.
    fn lived(&self) -> Lived {
        let home = home_of(&self.state, self.keeper);
        let worth = |moment: &HomeMoment| match moment {
            HomeMoment::Memento { .. } => 0,
            HomeMoment::AskedToMoveIn { .. } => 1,
            HomeMoment::StayedOver { .. } => 2,
            HomeMoment::Room { .. } => 3,
            HomeMoment::Favourite { .. } => 4,
            HomeMoment::Visit { .. } => 5,
            HomeMoment::Unknown => 6,
        };
        let mut moments: Vec<_> = home
            .journal
            .iter()
            .filter(|entry| entry.at_utc >= self.opened_at_utc)
            .filter(|entry| match &entry.moment {
                // A room built and taken back again is no news.
                HomeMoment::Room { room } => home
                    .rooms
                    .iter()
                    .any(|layout| layout.kind.as_ref() == Some(room)),
                HomeMoment::Unknown => false,
                _ => true,
            })
            .collect();
        moments.sort_by_key(|entry| (worth(&entry.moment), entry.at_utc));
        moments.truncate(formiga_home_contract::limits::MAX_MOMENTS);
        moments.sort_by_key(|entry| entry.at_utc);
        Lived {
            together: self
                .life
                .together()
                .into_iter()
                .map(|(a, b, how, times)| (TravelerId(a), TravelerId(b), how, times))
                .collect(),
            moments: moments
                .into_iter()
                .map(|entry| entry.moment.clone())
                .collect(),
            next_door: self.next_door,
            move_in: self.move_in.map(|friend| (TravelerId(friend), self.keeper)),
        }
    }

    /// Go over to the house `keeper` keeps. On a visit, the house is left and Desktop opens that
    /// one next, if it will; a rehearsal opens it itself, in the same window.
    fn go_next_door(&mut self, ctx: &egui::Context, keeper: TravelerId) {
        let name = self.household.snapshot.neighbour(keeper).map_or_else(
            || "the house next door".to_owned(),
            |house| format!("{}'s house", house.name),
        );
        self.next_door = Some(keeper);
        let (colony, label) = match &self.host {
            Host::Visit(_) => {
                self.say(format!("Off to {name}…"));
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            Host::Rehearsal(rehearsal) => (rehearsal.colony.clone(), rehearsal.label.clone()),
        };
        self.leave();
        let opened = colony
            .open(crate::host::Which::Kept(keeper))
            .and_then(|snapshot| {
                let household = Household::new(snapshot.clone())
                    .map_err(|error| anyhow::anyhow!("could not draw the household: {error}"))?;
                Ok((snapshot, household))
            });
        match opened {
            Ok((snapshot, household)) => {
                let homes = store::RehearsalHomes::new(self.data.as_deref(), &snapshot.colony_key);
                let host =
                    Host::Rehearsal(crate::host::Rehearsal::new(snapshot, homes, label, colony));
                let open = self._open.take();
                let data = self.data.clone();
                *self = HomeApp::new(ctx, household, host, data, open);
                self.say(format!("Over at {name}."));
            }
            Err(error) => {
                eprintln!("formiga-home: {error:#}");
                self.next_door = None;
                self.left = false;
                self.say(format!("{name} could not be opened."));
            }
        }
    }

    /// A new keepsake for the house, from `by` and of `of`, each of them as they look now,
    /// handed back to Desktop at once. Its id, if the house had room for it.
    fn make_keepsake(
        &mut self,
        kind: MementoKind,
        by: Option<Id>,
        of: Vec<Id>,
    ) -> Option<DisplayId> {
        let of: Vec<_> = of
            .iter()
            .filter_map(|id| self.household.resident(*id))
            .map(|resident| (TravelerId(resident.id), keepsakes::ink_of(resident)))
            .collect();
        let home = self.state.household_mut(self.keeper)?;
        let id = keepsakes::make(home, kind, by.map(TravelerId), of, now_utc())?;
        if let Some(memento) = home.memento(&id) {
            self.arranging.came(self.keeper, memento);
        }
        self.keepsakes_changed();
        self.keep();
        Some(id)
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
                let (cx, cy) = room::footprint(placed).centre();
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

    /// The house as a photo: everyone where they are, on the table-top light, and none of the
    /// window's own marks.
    fn photo(&mut self) -> Canvas {
        let now = self.now();
        let house = self.house_as_seen();
        let overlay = Overlay {
            lamps_off: self.life.lamps_off().to_vec(),
            backdrop: true,
            daylight: self.daylight(),
            ..Overlay::default()
        };
        self.scene.compose(
            &house,
            &self.household.snapshot,
            &mut self.life.actors,
            now,
            &overlay,
        )
    }

    /// Ask where to save a photo of the room, and save it there, three times the size.
    fn save_photo(&mut self) {
        let canvas = self.photo();
        let name = format!("{}.png", self.household.house_name());
        let chosen = rfd::FileDialog::new()
            .set_title("Save a picture of the room")
            .set_file_name(&name)
            .add_filter("PNG image", &["png"])
            .save_file();
        let Some(path) = chosen else { return };
        match crate::write_png(&path, &canvas, 3) {
            Ok(()) => {
                // And one framed for the house, of whoever was in it.
                let in_it: Vec<Id> = self
                    .life
                    .actors
                    .iter()
                    .filter(|actor| !actor.hidden && self.house.room_of_point(actor.pos).is_some())
                    .map(|actor| actor.id)
                    .collect();
                if self
                    .make_keepsake(MementoKind::Photo, None, in_it)
                    .is_some()
                {
                    self.say("Saved a picture of the room, and framed one for the drawer.");
                } else {
                    self.say("Saved a picture of the room.");
                }
            }
            Err(error) => self.say(format!("The picture could not be saved: {error}")),
        }
    }

    fn live_pointer(
        &mut self,
        response: &egui::Response,
        pointer: Option<(f32, f32)>,
        released: bool,
        now: f32,
    ) {
        if let Some(id) = self.carried {
            if let Some((x, y)) = pointer {
                let (fx, fy) = self.scene.view.floor_at(x, y + 10.0);
                let (w, d) = (f32::from(self.house.width), f32::from(self.house.depth));
                self.life
                    .carry(id, (fx.clamp(0.2, w - 0.2), fy.clamp(0.2, d - 0.2)));
            }
            if released || response.drag_stopped() {
                self.life.put_down(&self.house, id, now);
                self.carried = None;
            }
            return;
        }
        if response.drag_started()
            && let Some(Target::Resident(id)) = self.hovered.clone()
        {
            self.menu = None;
            self.life.pick_up(id, now);
            self.carried = Some(id);
            self.selected = Some(id);
            return;
        }
        // Dragged across anything else, a house seen close moves about under the pointer.
        if response.drag_started() {
            self.zoom.panning = self.zoom.closer_than_fits();
            return;
        }
        if response.secondary_clicked()
            && let Some(Target::Resident(id)) = self.hovered.clone()
        {
            self.life.pet(&self.household, id, now);
            return;
        }
        if !response.clicked() {
            return;
        }
        // A click away from an open menu only closes it.
        if self.menu.take().is_some() {
            return;
        }
        let Some(target) = self.hovered.clone() else {
            self.menu = None;
            return;
        };
        let at = response.interact_pointer_pos().unwrap_or_default();
        self.click_live(target, at);
    }

    /// Asking a visitor to stay over, while it is visiting and not staying already.
    fn stay_over_entry(&self, id: Id) -> Option<(Entry, String)> {
        (self.household.is_visitor(id)
            && self.life.present().contains(&id)
            && !self.life.staying(id))
        .then(|| {
            (
                Entry::StayOver(id),
                format!("Ask {} to stay over", self.name(id)),
            )
        })
    }

    /// Asking a visitor to move in, once a visit, where somebody will hear of it.
    fn move_in_entry(&self, id: Id) -> Option<(Entry, String)> {
        (self.household.is_visitor(id)
            && self.life.present().contains(&id)
            && self.move_in.is_none()
            && self.host.hears_move_ins())
        .then(|| {
            (
                Entry::MoveIn(id),
                format!("Ask {} to move in", self.name(id)),
            )
        })
    }

    /// A friend asked to come and live here: it says whether it would like to, and if so, Desktop
    /// hears of it on leaving and decides.
    fn ask_to_move_in(&mut self, visitor: Id) {
        let name = self.name(visitor);
        let warmth = self.household.friend_of(visitor).map_or(0.0, |friend| {
            life::band(self.household.bond(friend.id, visitor).warmth)
        });
        let Some(character) = self
            .household
            .resident(visitor)
            .map(|visitor| visitor.character.clone())
        else {
            return;
        };
        if !character.would_move_in(warmth) {
            self.say(format!("{name} is happy in their own house."));
            return;
        }
        self.move_in = Some(visitor);
        self.note(HomeMoment::AskedToMoveIn {
            visitor: TravelerId(visitor),
        });
        match &self.host {
            Host::Visit(_) => self.say(format!(
                "{name} would like that. Whether they move in is settled at home in the village."
            )),
            Host::Rehearsal(_) => self.say(format!(
                "{name} would like that, but nobody moves house in a rehearsal."
            )),
        }
    }

    fn click_live(&mut self, target: Target, at: egui::Pos2) {
        let now = self.now();
        let house = self.house.clone();
        let snapshot = &self.household.snapshot;
        match (target, self.selected) {
            (Target::Resident(id), Some(chosen)) if id == chosen => {
                self.menu = Some(Menu {
                    at,
                    title: self.name(id),
                    entries: {
                        let mut entries = vec![(Entry::Pet, "Give a pat".to_owned())];
                        entries.extend(self.stay_over_entry(id));
                        entries.extend(self.move_in_entry(id));
                        entries
                    },
                    opened: now,
                });
            }
            (Target::Resident(id), None) => {
                self.selected = Some(id);
                self.menu = None;
            }
            (Target::Floor(x, y), Some(chosen)) => {
                self.menu = None;
                self.ask(chosen, Act::GoTo(x, y));
            }
            (target, Some(chosen)) => {
                let present = self.life.present();
                let guests = self.life.guests();
                let acts = choices(
                    &self.household,
                    &house,
                    snapshot,
                    &present,
                    &guests,
                    chosen,
                    &target,
                );
                let mut entries: Vec<(Entry, String)> = acts
                    .into_iter()
                    .map(|act| {
                        let label = act.label(&self.household, &house, snapshot);
                        (Entry::Ask(act), label)
                    })
                    .collect();
                let title = match &target {
                    Target::Resident(other) => {
                        entries.extend(self.stay_over_entry(*other));
                        entries.extend(self.move_in_entry(*other));
                        entries.push((
                            Entry::Choose(*other),
                            format!("Choose {} instead", self.name(*other)),
                        ));
                        format!("{} and {}", self.name(chosen), self.name(*other))
                    }
                    Target::Shown(item) => snapshot
                        .item(item)
                        .map_or_else(String::new, |item| item.name.clone()),
                    Target::Piece(uid) => house
                        .piece(*uid)
                        .and_then(|placed| catalog::piece(&placed.piece))
                        .map_or_else(String::new, |piece| piece.name.to_owned()),
                    Target::Floor(..) => String::new(),
                };
                if entries.is_empty() {
                    self.say(format!("Nothing there for {} to do.", self.name(chosen)));
                    self.menu = None;
                } else {
                    self.menu = Some(Menu {
                        at,
                        title,
                        entries,
                        opened: now,
                    });
                }
            }
            (_, None) => {
                self.say("Click a resident first, to choose who to ask.");
            }
        }
    }

    fn arrange_pointer(
        &mut self,
        response: &egui::Response,
        pointer: Option<(f32, f32)>,
        released: bool,
    ) {
        if response.secondary_clicked() {
            self.arranging.turn();
            return;
        }
        // Picking something up from the house, by dragging it or by a click: a doorway first,
        // then what is pointed at.
        if self.arranging.carrying.is_none() && (response.drag_started() || response.clicked()) {
            let door = pointer
                .and_then(|point| arrange::wall_cell_at(&self.scene.view, &self.house, point))
                .filter(|wall| wall.door)
                .map(|wall| Carry::Door {
                    room: wall.room,
                    door: formiga_home_contract::Door {
                        side: wall.side,
                        at: wall.at,
                    },
                });
            let carry = door.or_else(|| match self.hovered.clone() {
                Some(Target::Piece(name)) => self.house.piece(name).map(|placed| Carry::Piece {
                    name,
                    turn: placed.turn,
                }),
                Some(Target::Shown(item)) => Some(Carry::Thing(item)),
                _ => None,
            });
            match carry {
                Some(carry) => {
                    self.arranging.carrying = Some(carry);
                    self.arranging.dragged = response.drag_started();
                }
                None if response.drag_started() => {
                    self.zoom.panning = self.zoom.closer_than_fits();
                }
                None => {}
            }
            return;
        }
        let Some(carrying) = self.arranging.carrying.clone() else {
            return;
        };
        let put_now = if self.arranging.dragged {
            released
        } else {
            response.clicked()
        };
        if !put_now {
            return;
        }
        let landing = pointer.and_then(|point| self.landing(point));
        let house = self.house.clone();
        let put = match landing {
            Some((landing, Some(_))) => self.arranging.put(
                &mut self.state,
                self.keeper,
                &self.household.snapshot,
                &house,
                landing,
            ),
            _ => false,
        };
        if put {
            let what = match carrying {
                Carry::Thing(_) => "Shown.",
                Carry::Door { .. } => "The doorway is moved.",
                _ => "Put down.",
            };
            self.changed(what);
        } else if pointer.is_some() {
            self.say("That will not go there.");
            if self.arranging.dragged {
                self.arranging.carrying = None;
            }
        } else {
            // Let go outside the room: it goes back where it came from.
            self.arranging.carrying = None;
        }
        self.arranging.dragged = false;
    }

    fn landing(&mut self, point: (f32, f32)) -> Option<(Landing, Option<Showing>)> {
        let now = self.now();
        let over = match &self.hovered {
            Some(Target::Piece(uid)) => Some(*uid),
            _ => None,
        };
        let over = over.or_else(|| {
            let pixel = (point.0 as i32, point.1 as i32);
            match self.scene.hit(
                &self.house,
                &self.household.snapshot,
                &mut self.life.actors,
                now,
                pixel,
            ) {
                Some(Target::Piece(uid)) => Some(uid),
                _ => None,
            }
        });
        self.arranging.landing(
            &self.scene.view,
            &self.house,
            home_of(&self.state, self.keeper),
            &self.household.snapshot,
            point,
            over,
        )
    }

    /// What is being carried, drawn where it would go: green where it fits, red where not. A
    /// doorway is shown as the stretch of wall it would go in.
    fn ghost(
        &mut self,
        pointer: Option<(f32, f32)>,
    ) -> (Option<Ghost>, Option<(crate::house::Wall, bool)>) {
        let Some(point) = pointer else {
            return (None, None);
        };
        let Some((landing, showing)) = self.landing(point) else {
            return (None, None);
        };
        let house = self.house.clone();
        if let Landing::Wall { room, door } = landing {
            let wall = house.wall(room, door.side, door.at).copied();
            return (None, wall.map(|wall| (wall, showing.is_some())));
        }
        if let Some((piece, turn, _)) = self.arranging.carried_piece(&house) {
            let Landing::Floor { room, x, y } = landing else {
                return (None, None);
            };
            let Some(at) = house.room(room) else {
                return (None, None);
            };
            let (x, y) = (at.x + x, at.y + y);
            let (w, d) = piece.size_at(turn);
            let sprite = self.scene.piece_sprite(piece, turn).clone();
            return (
                Some(Ghost {
                    sprite,
                    at: self.scene.view.pixel(f32::from(x), f32::from(y)),
                    fits: showing.is_some(),
                    footprint: Some(room::Footprint { x, y, w, d }),
                }),
                None,
            );
        }
        let Some(Carry::Thing(id)) = &self.arranging.carrying else {
            return (None, None);
        };
        let Some(item) = self.household.snapshot.item(id).cloned() else {
            return (None, None);
        };
        let Landing::Spot { room, spot } = landing else {
            return (None, None);
        };
        let Some(at) = house.at(room, spot) else {
            return (None, None);
        };
        let place = house.place_of(at).unwrap_or(Place::Top);
        let anchor = self
            .scene
            .spot_anchor(&house, at)
            .unwrap_or((point.0 as i32, point.1 as i32));
        let sprite = self
            .scene
            .thing(&item, place, showing.unwrap_or(Showing::Card))
            .clone();
        let footprint = match at {
            At::Floor { x, y } => Some(room::Footprint { x, y, w: 1, d: 1 }),
            _ => None,
        };
        (
            Some(Ghost {
                sprite,
                at: anchor,
                fits: showing.is_some(),
                footprint,
            }),
            None,
        )
    }

    fn menu(&mut self, ctx: &egui::Context) {
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

/// How many screen pixels to each of the house's picture fit its page: a whole number where at
/// least one does, so every pixel stays square.
fn fit_pixels(page: egui::Rect, pixels_per_point: f32, size: (u32, u32)) -> f32 {
    let fit = (page.width() * pixels_per_point / size.0 as f32)
        .min(page.height() * pixels_per_point / size.1 as f32);
    if fit >= 1.0 {
        fit.floor()
    } else {
        fit.max(0.1)
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

impl HomeApp {
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
}

impl HomeApp {
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

impl Drop for HomeApp {
    fn drop(&mut self) {
        self.leave();
    }
}

mod notebook;
mod pages;

pub use notebook::frameless;

#[cfg(test)]
mod tests;
