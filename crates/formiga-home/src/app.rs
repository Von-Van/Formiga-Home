//! The Home window: the room, scaled up whole, with the household living in it; a drawer beside
//! it of who is home, or of everything that can go in the room; and the two modes.
//!
//! The window opens in Live Mode, where the owner chooses a resident, then clicks something in
//! the room to see what it could do there; a resident can be carried by dragging it and given a
//! pat with a right-click. Arrange Mode is asked for explicitly, so a chair is never dragged by
//! accident: there everyone waits while furniture and finds are picked up and put down.

use crate::arrange::{self, Arranging, Carry, Landing};
use crate::art::displays;
use crate::catalog::{self, Arrival, FLOORS, Family, PIECES, WALLS};
use crate::host::Host;
use crate::household::{Household, Id, Whereabouts};
use crate::iso::{SCENE_HEIGHT, SCENE_WIDTH};
use crate::life::{Act, Asked, Life, QUEUE_LIMIT, choices};
use crate::room::{self, Place, Showing};
use crate::scene::{Ghost, Overlay, Scene, Target};
use crate::store::{self, WindowPlace};
use eframe::egui;
use formiga_art::Canvas;
use formiga_home_contract::{DisplayId, HomeState, RoomLayout, Spot, TravelerId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const NOTICE_SECS: f32 = 4.0;
/// Round the room, the colour the scene's own backdrop settles to.
const LETTERBOX: egui::Color32 = egui::Color32::from_rgb(0xd6, 0xc7, 0xb1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Live,
    Arrange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drawer {
    Furniture,
    Finds,
    Room,
}

/// What can be chosen from the menu over the room.
#[derive(Clone, Debug, PartialEq)]
enum Entry {
    Ask(Act),
    Pet,
    Choose(Id),
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
    scene: Scene,
    life: Life,
    mode: Mode,
    drawer: Drawer,
    selected: Option<Id>,
    hovered: Option<Target>,
    menu: Option<Menu>,
    arranging: Arranging,
    carried: Option<Id>,
    notice: Option<(String, f32)>,
    texture: Option<egui::TextureHandle>,
    thumbnails: HashMap<String, egui::TextureHandle>,
    /// Seconds since the window opened, as egui's input says: one clock for the whole frame.
    clock: f32,
    /// Where the room was drawn last frame.
    room_rect: Option<egui::Rect>,
    last: f32,
    last_recall_check: f32,
    data: Option<PathBuf>,
    place: Option<WindowPlace>,
    left: bool,
    _open: Option<store::Open>,
}

/// The visited household's first room in `state`.
fn layout_of(state: &HomeState, keeper: TravelerId) -> &RoomLayout {
    &state
        .household(keeper)
        .expect("the visited household always has a home")
        .rooms[0]
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
        // Names in the drawer are things to pick up, not text to select: a selectable label
        // would take the click and the drag for itself.
        ctx.all_styles_mut(|style| style.interaction.selectable_labels = false);
        ctx.set_zoom_factor(f32::from(presentation.text_scale_percent.clamp(100, 150)) / 100.0);
        let mut state = host.state().clone();
        arrange::ensure_home(&mut state, &household.snapshot);
        let keeper = household.snapshot.household.keeper;
        let layout = layout_of(&state, keeper);
        let scene = Scene::new(layout);
        let life = Life::new(&household, layout);
        Self {
            selected: household.residents.first().map(|resident| resident.id),
            household,
            host,
            state,
            keeper,
            scene,
            life,
            mode: Mode::Live,
            drawer: Drawer::Finds,
            hovered: None,
            menu: None,
            arranging: Arranging::default(),
            carried: None,
            notice: None,
            texture: None,
            thumbnails: HashMap::new(),
            clock: 0.0,
            room_rect: None,
            last: 0.0,
            last_recall_check: 0.0,
            place: data.as_deref().and_then(WindowPlace::load),
            data,
            left: false,
            _open: open,
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
        if let Err(error) = self.host.leave(&self.state) {
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
            self.life
                .put_down(layout_of(&self.state, self.keeper), id, now);
        }
        match mode {
            Mode::Arrange => {
                self.life
                    .pause(&self.household, layout_of(&self.state, self.keeper), now);
            }
            Mode::Live => {
                self.arranging.carrying = None;
                self.arranging.dragged = false;
                self.life.resume(
                    layout_of(&self.state, self.keeper),
                    &self.household.snapshot,
                    now,
                );
                self.keep();
            }
        }
        self.mode = mode;
    }

    fn name(&self, id: Id) -> String {
        self.household
            .resident(id)
            .map_or_else(|| "Someone".to_owned(), |resident| resident.name.clone())
    }

    fn ask(&mut self, id: Id, act: Act) {
        let now = self.now();
        let label = act.label(
            &self.household,
            layout_of(&self.state, self.keeper),
            &self.household.snapshot,
        );
        let name = self.name(id);
        match self.life.ask(id, act, now) {
            Asked::Queued => self.say(format!("{name}: {}", label.to_lowercase())),
            Asked::Full => self.say(format!(
                "{name} has {QUEUE_LIMIT} things to do already. Take one back in the drawer, or wait."
            )),
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        let (escape, turn, away, undo, redo) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
                command && !input.modifiers.shift && input.key_pressed(egui::Key::Z),
                command && input.modifiers.shift && input.key_pressed(egui::Key::Z),
            )
        });
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if escape {
            if self.menu.take().is_none() && self.arranging.carrying.take().is_none() {
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
        if away && self.arranging.put_away(&mut self.state, self.keeper) {
            self.changed("Put away.");
        }
        if undo && self.arranging.undo(&mut self.state) {
            self.changed("Undone.");
        }
        if redo && self.arranging.redo(&mut self.state) {
            self.changed("Done again.");
        }
    }

    /// The room has changed: everyone steps clear of the furniture, and Desktop is handed the
    /// homes as they now stand.
    fn changed(&mut self, notice: &str) {
        self.life.make_room(layout_of(&self.state, self.keeper));
        self.keep();
        self.say(notice);
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading(self.household.house_name());
            ui.separator();
            let mut mode = self.mode;
            ui.selectable_value(&mut mode, Mode::Live, "Live");
            ui.selectable_value(&mut mode, Mode::Arrange, "Arrange");
            if mode != self.mode {
                self.set_mode(mode);
            }
            if self.mode == Mode::Arrange {
                ui.separator();
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
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Leave the house").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if let Some(label) = self.host.rehearsal_label() {
                    ui.weak(label);
                }
            });
        });
        ui.add_space(4.0);
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let layout = layout_of(&self.state, self.keeper);
        let snapshot = &self.household.snapshot;
        let line = match self.selected {
            Some(id) => self.life.doing(&self.household, layout, snapshot, id),
            None => self
                .household
                .residents
                .iter()
                .map(|resident| {
                    self.life
                        .doing(&self.household, layout, snapshot, resident.id)
                })
                .collect::<Vec<_>>()
                .join(" "),
        };
        ui.horizontal(|ui| {
            ui.label(line);
            if let Some((notice, _)) = &self.notice {
                ui.separator();
                ui.label(egui::RichText::new(notice).italics());
            }
        });
        ui.add_space(4.0);
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

    fn drawer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(6.0);
                match self.mode {
                    Mode::Live => self.drawer_live(ui),
                    Mode::Arrange => {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.drawer, Drawer::Finds, "Found things");
                            ui.selectable_value(&mut self.drawer, Drawer::Furniture, "Furniture");
                            ui.selectable_value(&mut self.drawer, Drawer::Room, "Room");
                        });
                        ui.separator();
                        match self.drawer {
                            Drawer::Furniture => self.drawer_furniture(ui, ctx),
                            Drawer::Finds => self.drawer_finds(ui, ctx),
                            Drawer::Room => self.drawer_room(ui),
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.weak(
                            "Drag something, or click it, to pick it up; click to put it down. \
                         R or a right-click turns a piece, Delete puts it away, and ⌘Z undoes.",
                        );
                    }
                }
            });
    }

    fn drawer_live(&mut self, ui: &mut egui::Ui) {
        ui.strong("Who is home");
        let now = self.now();
        let layout = layout_of(&self.state, self.keeper).clone();
        let snapshot = self.household.snapshot.clone();
        let ids: Vec<Id> = self.household.residents.iter().map(|r| r.id).collect();
        for id in ids {
            let resident = self.household.resident(id).expect("a resident");
            let label = if resident.is_little() {
                format!("{} (little one)", resident.name)
            } else {
                resident.name.clone()
            };
            let phrase = resident.traveler.character.phrase.clone();
            if ui
                .selectable_label(self.selected == Some(id), label)
                .clicked()
            {
                self.selected = if self.selected == Some(id) {
                    None
                } else {
                    Some(id)
                };
                self.menu = None;
            }
            ui.weak(phrase);
            ui.add_space(4.0);
        }
        ui.separator();
        if let Some(id) = self.selected {
            ui.strong(format!("Asked of {}", self.name(id)));
            let queue = self.life.queue(id);
            if queue.is_empty() {
                ui.weak("Nothing. Click something in the room to see what they could do there.");
            }
            let mut cancel = None;
            for (position, act) in queue.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .small_button("✕")
                        .on_hover_text("Take this back")
                        .clicked()
                    {
                        cancel = Some(position);
                    }
                    ui.label(act.label(&self.household, &layout, &snapshot));
                });
            }
            if let Some(position) = cancel {
                self.life.cancel(id, position, now);
            }
            ui.add_space(6.0);
        }
        ui.separator();
        ui.weak(
            "Click a resident to choose them, then click a seat, a toy, a find or another \
             resident to see what they could do. Click the floor to send them there. Drag a \
             resident to carry them; right-click for a pat. When nothing is asked, everyone \
             does as they please.",
        );
    }

    fn drawer_furniture(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let snapshot = &self.household.snapshot;
        let (days, things) = (snapshot.days_lived, snapshot.inventory.len());
        let layout = layout_of(&self.state, self.keeper).clone();
        for family in Family::ALL {
            let pieces: Vec<&'static catalog::Piece> =
                PIECES.iter().filter(|p| p.family == family).collect();
            if pieces.is_empty() {
                continue;
            }
            ui.add_space(4.0);
            ui.strong(family.label());
            for piece in pieces {
                let available = piece.available(days, things);
                let (texture, size) = self.thumbnail(ctx, format!("piece:{}", piece.id), || {
                    crate::art::furniture::draw(piece, false).canvas
                });
                let count = layout
                    .pieces
                    .iter()
                    .filter(|placed| placed.piece.as_str() == piece.id)
                    .count();
                let response = ui
                    .horizontal(|ui| {
                        let scale = (40.0 / size.y.max(1.0)).min(1.5);
                        ui.add(
                            egui::Image::new((texture, size * scale)).tint(if available {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_white_alpha(90)
                            }),
                        );
                        ui.vertical(|ui| {
                            if available {
                                ui.label(piece.name);
                            } else {
                                ui.weak(piece.name);
                            }
                            match (available, piece.arrives, count) {
                                (false, Arrival::AfterDays(_) | Arrival::AfterFinds(_), _) => {
                                    ui.weak("Arrives in time");
                                }
                                (true, _, 0) => {}
                                (true, _, count) => {
                                    ui.weak(format!("{count} in the room"));
                                }
                                _ => {}
                            }
                        });
                    })
                    .response
                    .interact(egui::Sense::click_and_drag());
                if available && (response.clicked() || response.drag_started()) {
                    self.arranging.carrying = Some(Carry::New { piece, turn: 0 });
                    self.arranging.dragged = response.drag_started();
                }
                if available {
                    response.on_hover_cursor(egui::CursorIcon::Grab);
                }
            }
        }
    }

    fn drawer_finds(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let snapshot = self.household.snapshot.clone();
        if snapshot.inventory.is_empty() {
            ui.weak("Nothing found yet. Whatever the colony finds will be here to show.");
            return;
        }
        let layout = layout_of(&self.state, self.keeper).clone();
        let mut put_away = None;
        for item in &snapshot.inventory {
            let (texture, size) =
                self.thumbnail(ctx, format!("item:{}", item.id), || displays::icon(item));
            let whereabouts = match self.household.whereabouts(&self.state, &item.id) {
                Whereabouts::Nowhere => "Not shown anywhere yet".to_owned(),
                Whereabouts::Here => format!("Here, {}", where_here(&layout, &item.id)),
                Whereabouts::Elsewhere(house) => format!("In {house}"),
            };
            let here = matches!(
                self.household.whereabouts(&self.state, &item.id),
                Whereabouts::Here
            );
            let response = ui
                .horizontal(|ui| {
                    let scale = 2.0_f32.min(32.0 / size.x.max(size.y).max(1.0) * 2.0);
                    ui.add(egui::Image::new((texture, size * scale)));
                    ui.vertical(|ui| {
                        ui.label(&item.name);
                        ui.weak(whereabouts);
                        if let Some(finder) = &item.finder_name {
                            ui.weak(format!("Found by {finder}"));
                        }
                    });
                })
                .response
                .interact(egui::Sense::click_and_drag());
            if response.clicked() || response.drag_started() {
                self.arranging.carrying = Some(Carry::Thing(item.id.clone()));
                self.arranging.dragged = response.drag_started();
            }
            let response = response.on_hover_cursor(egui::CursorIcon::Grab);
            if here {
                response.context_menu(|ui| {
                    if ui.button("Put away").clicked() {
                        put_away = Some(item.id.clone());
                        ui.close();
                    }
                });
            }
            ui.add_space(2.0);
        }
        if let Some(item) = put_away {
            self.arranging.carrying = Some(Carry::Thing(item));
            if self.arranging.put_away(&mut self.state, self.keeper) {
                self.changed("Back in the drawer.");
            }
        }
    }

    fn drawer_room(&mut self, ui: &mut egui::Ui) {
        let layout = layout_of(&self.state, self.keeper).clone();
        ui.strong("Floor");
        let mut floor = None;
        for finish in &FLOORS {
            if ui
                .radio(layout.floor.as_str() == finish.id, finish.name)
                .clicked()
            {
                floor = Some(finish.id);
            }
        }
        ui.add_space(6.0);
        ui.strong("Walls");
        let mut wall = None;
        for finish in &WALLS {
            if ui
                .radio(layout.wall.as_str() == finish.id, finish.name)
                .clicked()
            {
                wall = Some(finish.id);
            }
        }
        if floor.is_some() || wall.is_some() {
            self.arranging
                .finish(&mut self.state, self.keeper, floor, wall);
            self.changed("The room has a new look.");
        }
    }

    fn room_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let now = self.now();
        let available = ui.available_rect_before_wrap();
        let rect = scene_rect(available, ctx.pixels_per_point());
        self.room_rect = Some(rect);
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        let scale = rect.width() / SCENE_WIDTH as f32;
        // While something is carried the pointer is followed wherever it is pressed from, the
        // drawer included; otherwise only while it is over the room.
        let carrying = self.carried.is_some() || self.arranging.dragged;
        let pointer = if carrying {
            ctx.input(|input| input.pointer.latest_pos())
        } else {
            response.hover_pos()
        }
        .filter(|pos| rect.contains(*pos))
        .map(|pos| ((pos.x - rect.min.x) / scale, (pos.y - rect.min.y) / scale));
        let pixel = pointer.map(|(x, y)| (x as i32, y as i32));
        let released = ctx.input(|input| input.pointer.primary_released());

        let keeper = self.keeper;
        let snapshot = self.household.snapshot.clone();
        let layout = layout_of(&self.state, keeper).clone();
        if self.menu.is_none() {
            self.hovered = pixel.and_then(|point| {
                self.scene
                    .hit(&layout, &snapshot, &mut self.life.actors, now, point)
            });
        }

        let mut overlay = Overlay {
            hovered: self.hovered.clone(),
            selected: self.selected,
            arranging: self.mode == Mode::Arrange,
            ..Overlay::default()
        };
        match self.mode {
            Mode::Live => self.live_pointer(&response, pointer, released, &layout, now),
            Mode::Arrange => {
                self.arrange_pointer(&response, pointer, released, &layout);
                overlay.ghost = self.ghost(&layout, pointer);
                overlay.lifted = match &self.arranging.carrying {
                    Some(Carry::Piece { uid, .. }) => Some(Target::Piece(*uid)),
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

        let layout = layout_of(&self.state, keeper).clone();
        let canvas = self
            .scene
            .compose(&layout, &snapshot, &mut self.life.actors, now, &overlay);
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [SCENE_WIDTH as usize, SCENE_HEIGHT as usize],
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
        let painter = ui.painter_at(rect);
        painter.image(
            texture,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        // The hovered resident's name, in a tag over its head.
        if let Some(Target::Resident(id)) = &self.hovered
            && self.mode == Mode::Live
            && let Some(actor) = self.life.actors.iter_mut().find(|actor| actor.id == *id)
        {
            let (l, t, r, _) = actor.bounds(&self.scene.view, now);
            let at = rect.min + egui::vec2((l + r) as f32 / 2.0, t as f32 - 9.0) * scale;
            let name = self.household.resident(*id).map_or("", |r| r.name.as_str());
            let font = egui::FontId::proportional(13.0);
            let galley = painter.layout_no_wrap(
                name.to_owned(),
                font,
                egui::Color32::from_rgb(0x3a, 0x2a, 0x24),
            );
            let tag = egui::Rect::from_center_size(at, galley.size() + egui::vec2(10.0, 4.0));
            painter.rect_filled(
                tag,
                4.0,
                egui::Color32::from_rgba_unmultiplied(0xfb, 0xf4, 0xe6, 230),
            );
            painter.galley(
                tag.center() - galley.size() / 2.0,
                galley,
                egui::Color32::BLACK,
            );
        }
    }

    fn live_pointer(
        &mut self,
        response: &egui::Response,
        pointer: Option<(f32, f32)>,
        released: bool,
        layout: &RoomLayout,
        now: f32,
    ) {
        if let Some(id) = self.carried {
            if let Some((x, y)) = pointer {
                let (fx, fy) = self.scene.view.floor_at(x, y + 10.0);
                let (w, d) = (f32::from(layout.width), f32::from(layout.depth));
                self.life
                    .carry(id, (fx.clamp(0.2, w - 0.2), fy.clamp(0.2, d - 0.2)));
            }
            if released || response.drag_stopped() {
                self.life.put_down(layout, id, now);
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
        self.click_live(target, at, layout);
    }

    fn click_live(&mut self, target: Target, at: egui::Pos2, layout: &RoomLayout) {
        let now = self.now();
        let snapshot = &self.household.snapshot;
        match (target, self.selected) {
            (Target::Resident(id), Some(chosen)) if id == chosen => {
                self.menu = Some(Menu {
                    at,
                    title: self.name(id),
                    entries: vec![(Entry::Pet, "Give a pat".to_owned())],
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
                let acts = choices(&self.household, layout, snapshot, chosen, &target);
                let mut entries: Vec<(Entry, String)> = acts
                    .into_iter()
                    .map(|act| {
                        let label = act.label(&self.household, layout, snapshot);
                        (Entry::Ask(act), label)
                    })
                    .collect();
                let title = match &target {
                    Target::Resident(other) => {
                        entries.push((
                            Entry::Choose(*other),
                            format!("Choose {} instead", self.name(*other)),
                        ));
                        format!("{} and {}", self.name(chosen), self.name(*other))
                    }
                    Target::Shown(item) => snapshot
                        .item(item)
                        .map_or_else(String::new, |item| item.name.clone()),
                    Target::Piece(uid) => layout
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
        layout: &RoomLayout,
    ) {
        if response.secondary_clicked() {
            self.arranging.turn();
            return;
        }
        // Picking something up from the room, by dragging it or by a click.
        if self.arranging.carrying.is_none() && (response.drag_started() || response.clicked()) {
            let carry = match self.hovered.clone() {
                Some(Target::Piece(uid)) => layout.piece(uid).map(|placed| Carry::Piece {
                    uid,
                    turn: placed.turn,
                }),
                Some(Target::Shown(item)) => Some(Carry::Thing(item)),
                _ => None,
            };
            if let Some(carry) = carry {
                self.arranging.carrying = Some(carry);
                self.arranging.dragged = response.drag_started();
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
        let landing = pointer.and_then(|point| self.landing(layout, point));
        let put = match landing {
            Some((landing, Some(_))) => self.arranging.put(
                &mut self.state,
                self.keeper,
                &self.household.snapshot,
                landing,
            ),
            _ => false,
        };
        if put {
            let what = match carrying {
                Carry::Thing(_) => "Shown.",
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

    fn landing(
        &mut self,
        layout: &RoomLayout,
        point: (f32, f32),
    ) -> Option<(Landing, Option<Showing>)> {
        let over = match &self.hovered {
            Some(Target::Piece(uid)) => Some(*uid),
            _ => None,
        };
        let now = self.now();
        let over = over.or_else(|| {
            let pixel = (point.0 as i32, point.1 as i32);
            match self.scene.hit(
                layout,
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
            layout,
            &self.household.snapshot,
            point,
            over,
        )
    }

    /// What is being carried, drawn where it would go: green where it fits, red where not.
    fn ghost(&mut self, layout: &RoomLayout, pointer: Option<(f32, f32)>) -> Option<Ghost> {
        let point = pointer?;
        let (landing, showing) = self.landing(layout, point)?;
        if let Some((piece, turn, _)) = self.arranging.carried_piece(layout) {
            let Landing::Floor { x, y } = landing else {
                return None;
            };
            let (w, d) = piece.size_at(turn);
            let sprite = self.scene.piece_sprite(piece, turn).clone();
            return Some(Ghost {
                sprite,
                at: self.scene.view.pixel(f32::from(x), f32::from(y)),
                fits: showing.is_some(),
                footprint: Some(room::Footprint { x, y, w, d }),
            });
        }
        let Some(Carry::Thing(id)) = &self.arranging.carrying else {
            return None;
        };
        let item = self.household.snapshot.item(id)?.clone();
        let Landing::Spot(spot) = landing else {
            return None;
        };
        let place = room::place_of(layout, spot).unwrap_or(Place::Top);
        let at = self
            .scene
            .spot_anchor(layout, spot)
            .unwrap_or((point.0 as i32, point.1 as i32));
        let sprite = self
            .scene
            .thing(&item, place, showing.unwrap_or(Showing::Card))
            .clone();
        let footprint = match spot {
            Spot::Floor { x, y } => Some(room::Footprint { x, y, w: 1, d: 1 }),
            _ => None,
        };
        Some(Ghost {
            sprite,
            at,
            fits: showing.is_some(),
            footprint,
        })
    }

    fn menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = &self.menu else { return };
        let mut chosen = None;
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("actions"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.at + egui::vec2(8.0, 8.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(150.0);
                    if !menu.title.is_empty() {
                        ui.strong(&menu.title);
                        ui.separator();
                    }
                    for (entry, label) in &menu.entries {
                        if ui.button(label).clicked() {
                            chosen = Some(entry.clone());
                        }
                    }
                    ui.separator();
                    if ui.button("Never mind").clicked() {
                        close = true;
                    }
                });
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
            }
        } else if close || clicked_elsewhere {
            self.menu = None;
        }
    }
}

/// Where in this house a thing is shown, as the drawer says it: "on the open shelf".
fn where_here(layout: &RoomLayout, item: &DisplayId) -> String {
    let Some(shown) = layout.displays.iter().find(|shown| &shown.item == item) else {
        return "in another room".to_owned();
    };
    match shown.spot {
        Spot::On { piece, .. } => layout
            .piece(piece)
            .and_then(|placed| catalog::piece(&placed.piece))
            .map_or_else(
                || "on a shelf".to_owned(),
                |piece| format!("on the {}", piece.name.to_lowercase()),
            ),
        Spot::Wall { .. } => "on the wall".to_owned(),
        Spot::Floor { .. } => "on the floor".to_owned(),
    }
}

/// The room scaled up by a whole number of pixels where it fits, so every pixel stays square.
fn scene_rect(available: egui::Rect, pixels_per_point: f32) -> egui::Rect {
    let fit = (available.width() * pixels_per_point / SCENE_WIDTH as f32)
        .min(available.height() * pixels_per_point / SCENE_HEIGHT as f32);
    let pixels = if fit >= 1.0 {
        fit.floor()
    } else {
        fit.max(0.1)
    };
    let size = egui::vec2(SCENE_WIDTH as f32, SCENE_HEIGHT as f32) * pixels / pixels_per_point;
    egui::Rect::from_center_size(available.center(), size)
}

impl eframe::App for HomeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
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
            let layout = layout_of(&self.state, self.keeper);
            self.life
                .tick(&self.household, layout, &self.household.snapshot, now, dt);
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, since)| now - since > NOTICE_SECS)
        {
            self.notice = None;
        }
        egui::Panel::top("bar").show(ui, |ui| self.top_bar(ui, &ctx));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::right("drawer")
            .resizable(false)
            .exact_size(236.0)
            .show(ui, |ui| self.drawer(ui, &ctx));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(LETTERBOX))
            .show(ui, |ui| self.room_view(ui, &ctx));
        self.menu(&ctx);
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}

impl Drop for HomeApp {
    fn drop(&mut self) {
        self.leave();
    }
}

#[cfg(test)]
mod tests;
