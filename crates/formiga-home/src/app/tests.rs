//! The window driven as a person would drive it: frames run headless through egui with pointer
//! and key events, things in the room found by asking the scene what is under the pointer, and
//! what each click and drag did read back from the household and the homes.

use super::*;
use crate::host::Rehearsal;
use crate::store::RehearsalHomes;
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use formiga_home_contract::{DisplayId, RoomLayout, Spot, sample};
use std::path::Path;

struct Harness {
    ctx: egui::Context,
    app: HomeApp,
    time: f64,
    output: Option<egui::FullOutput>,
    /// Everything the window has asked of the system, frame by frame.
    commands: Vec<egui::ViewportCommand>,
}

/// A scratch data folder of its own for each test.
fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("formiga-home-window-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

impl Harness {
    fn open(data: &Path) -> Self {
        let snapshot = sample::snapshot();
        let household = Household::new(snapshot.clone()).unwrap();
        let homes = RehearsalHomes::new(Some(data), &snapshot.colony_key);
        let host = Host::Rehearsal(Rehearsal::new(
            snapshot,
            homes,
            "test".to_owned(),
            crate::host::Colony::Sample,
        ));
        let ctx = egui::Context::default();
        let app = HomeApp::new(&ctx, household, host, Some(data.to_owned()), None);
        let mut harness = Self {
            ctx,
            app,
            time: 0.0,
            output: None,
            commands: Vec::new(),
        };
        harness.wait(0.2);
        harness
    }

    fn step(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 800.0))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame(ui));
        // No renderer here to hand the textures to.
        output.textures_delta.clear();
        for viewport in output.viewport_output.values() {
            self.commands.extend(viewport.commands.iter().cloned());
        }
        self.output = Some(output);
        self.time += 1.0 / 30.0;
    }

    fn wait(&mut self, seconds: f64) {
        let until = self.time + seconds;
        while self.time < until {
            self.step(Vec::new());
        }
    }

    fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn click_with(&mut self, pos: Pos2, button: PointerButton) {
        self.step(vec![Event::PointerMoved(pos)]);
        self.step(vec![Self::button(pos, button, true)]);
        self.step(vec![Self::button(pos, button, false)]);
        self.step(Vec::new());
    }

    fn click(&mut self, pos: Pos2) {
        self.click_with(pos, PointerButton::Primary);
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.step(vec![Event::PointerMoved(from)]);
        self.step(vec![Self::button(from, PointerButton::Primary, true)]);
        for n in 1..=12 {
            let along = n as f32 / 12.0;
            self.step(vec![Event::PointerMoved(from + (to - from) * along)]);
        }
        self.step(vec![Self::button(to, PointerButton::Primary, false)]);
        self.step(Vec::new());
    }

    fn key(&mut self, key: Key) {
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.step(vec![event(true)]);
        self.step(vec![event(false)]);
    }

    /// Where a piece of text was drawn last frame: a button, a tab, a name in the drawer.
    fn text(&self, wanted: &str) -> Option<Pos2> {
        fn search(shape: &egui::Shape, wanted: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == wanted => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| search(shape, wanted)),
                _ => None,
            }
        }
        self.output
            .as_ref()?
            .shapes
            .iter()
            .find_map(|clipped| search(&clipped.shape, wanted))
    }

    /// Where a control with its own id was laid out last frame: a tab, a tile, a stud.
    fn at_id(&self, id: egui::Id) -> Pos2 {
        self.ctx
            .read_response(id)
            .unwrap_or_else(|| panic!("nothing answers to {id:?}"))
            .rect
            .center()
    }

    fn click_id(&mut self, id: egui::Id) {
        let at = self.at_id(id);
        self.click(at);
    }

    fn arrange(&mut self) {
        self.click_id(notebook::mode_id(Mode::Arrange));
    }

    fn turn_to(&mut self, page: Drawer) {
        self.click_id(notebook::page_id(page));
    }

    fn click_text(&mut self, wanted: &str) {
        let at = self
            .text(wanted)
            .unwrap_or_else(|| panic!("nothing says {wanted:?}"));
        self.click(at);
    }

    /// A point of the scene, on the window.
    fn on_window(&self, (x, y): (f32, f32)) -> Pos2 {
        let rect = self.app.room_rect.expect("the room has been drawn");
        let scale = rect.width() / self.app.scene.view.size.0 as f32;
        rect.min + egui::vec2(x + 0.5, y + 0.5) * scale
    }

    /// Somewhere on the window the room shows `target` under the pointer.
    fn find(&mut self, target: &Target) -> Pos2 {
        let view = self.app.scene.view;
        let layout = self.app.house.clone();
        let (cx, cy) = match target {
            Target::Floor(x, y) => view.tile_centre(*x, *y),
            Target::Piece(uid) => {
                let footprint = placement::footprint(layout.piece(*uid).unwrap());
                let (x, y) = footprint.centre();
                let (sx, sy) = view.screen(x, y);
                (sx, sy - 6.0)
            }
            Target::Resident(id) => {
                let now = self.app.now();
                let actor = self
                    .app
                    .life
                    .actors
                    .iter_mut()
                    .find(|a| a.id == *id)
                    .unwrap();
                let (l, t, r, b) = actor.bounds(&view, now);
                ((l + r) as f32 / 2.0, (t + b) as f32 / 2.0)
            }
            Target::Shown(item) => {
                let at = layout.shown.iter().find(|s| &s.item == item).unwrap().at;
                let (x, y) = self.app.scene.spot_anchor(&layout, at).unwrap();
                (x as f32, y as f32 - 3.0)
            }
        };
        let now = self.app.now();
        for ring in 0_i32..14 {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dy.abs()) != ring {
                        continue;
                    }
                    let point = (cx + dx as f32 * 2.0, cy + dy as f32 * 2.0);
                    let hit = self.app.scene.hit(
                        &layout,
                        &self.app.household.snapshot,
                        &mut self.app.life.actors,
                        now,
                        (point.0 as i32, point.1 as i32),
                    );
                    if hit.as_ref() == Some(target) {
                        return self.on_window(point);
                    }
                }
            }
        }
        panic!("{target:?} is nowhere to be pointed at")
    }

    fn piece(&self, id: &str) -> u16 {
        self.layout()
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == id)
            .unwrap_or_else(|| panic!("no {id} in the room"))
            .uid
    }

    fn keeper(&self) -> Id {
        self.app.household.residents[0].id
    }

    fn pip(&self) -> Id {
        self.app.household.residents[1].id
    }

    /// The house's first room, as the state keeps it. Its pieces' names are the house's own.
    fn layout(&self) -> RoomLayout {
        home_of(&self.app.state, self.app.keeper).rooms[0].clone()
    }
}

#[test]
fn a_seat_chosen_from_the_menu_is_asked_for_and_then_sat_in() {
    let data = scratch("seat");
    let mut window = Harness::open(&data);
    let keeper = window.keeper();
    assert_eq!(
        window.app.selected,
        Some(keeper),
        "the keeper is chosen to begin with"
    );
    let chair = window.piece("armchair");
    let at = window.find(&Target::Piece(chair));
    window.click(at);
    assert!(
        window.app.menu.is_some(),
        "clicking the armchair opens its menu"
    );
    window.click_text("Sit on the armchair");
    assert!(window.app.menu.is_none());
    assert!(
        window.app.life.queue(keeper).contains(&Act::Sit(chair)),
        "{:?}",
        window.app.life.queue(keeper)
    );
    let mut sat = false;
    for _ in 0..80 {
        window.wait(0.25);
        if window.app.life.actor(keeper).unwrap().on_piece == Some(chair) {
            sat = true;
            break;
        }
    }
    assert!(sat, "the keeper never sat in the armchair");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_click_on_the_floor_sends_the_chosen_resident_there() {
    let data = scratch("floor");
    let mut window = Harness::open(&data);
    let keeper = window.keeper();
    let at = window.find(&Target::Floor(6, 6));
    window.click(at);
    assert_eq!(window.app.life.queue(keeper), vec![Act::GoTo(6, 6)]);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn another_resident_can_be_greeted_and_the_chosen_one_given_a_pat() {
    let data = scratch("social");
    let mut window = Harness::open(&data);
    let (keeper, pip) = (window.keeper(), window.pip());
    let at = window.find(&Target::Resident(pip));
    window.click(at);
    window.click_text("Say hello to Pip");
    assert!(window.app.life.queue(keeper).contains(&Act::Greet(pip)));
    let at = window.find(&Target::Resident(keeper));
    window.click(at);
    window.click_text("Give a pat");
    assert!(
        window.app.life.queue(keeper).is_empty(),
        "a pat lets go of what was asked"
    );
    let layout = window.app.house.clone();
    let doing = window.app.life.doing(
        &window.app.household,
        &layout,
        &window.app.household.snapshot,
        keeper,
    );
    assert!(doing.contains("pat"), "{doing}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_click_away_from_a_menu_closes_it_and_asks_for_nothing() {
    let data = scratch("away");
    let mut window = Harness::open(&data);
    let keeper = window.keeper();
    let chair = window.piece("armchair");
    let at = window.find(&Target::Piece(chair));
    window.click(at);
    assert!(window.app.menu.is_some());
    // Clear of the menu, which opens beside the armchair.
    let floor = window.find(&Target::Floor(7, 1));
    window.click(floor);
    assert!(window.app.menu.is_none());
    assert!(window.app.life.queue(keeper).is_empty());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_resident_right_clicked_gets_a_pat_and_one_dragged_is_set_down_on_open_floor() {
    let data = scratch("handling");
    let mut window = Harness::open(&data);
    let pip = window.pip();
    let at = window.find(&Target::Resident(pip));
    window.click_with(at, PointerButton::Secondary);
    let layout = window.app.house.clone();
    let doing = window.app.life.doing(
        &window.app.household,
        &layout,
        &window.app.household.snapshot,
        pip,
    );
    assert!(doing.contains("pat"), "{doing}");
    window.wait(3.0);
    let from = window.find(&Target::Resident(pip));
    let to = window.on_window(window.app.scene.view.tile_centre(6, 6));
    window.drag(from, to);
    assert!(window.app.carried.is_none(), "let go");
    let pos = window.app.life.actor(pip).unwrap().pos;
    let tile = crate::path::Floor::tile_of(pos);
    assert!(crate::path::Floor::of(&layout).open(tile.0, tile.1));
    assert!(
        (tile.0 - 6).abs() <= 1 && (tile.1 - 6).abs() <= 1,
        "{tile:?}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_find_from_the_drawer_goes_on_the_shelf_and_can_be_undone_and_redone() {
    let data = scratch("shelf");
    let mut window = Harness::open(&data);
    window.arrange();
    assert_eq!(window.app.mode, Mode::Arrange);
    window.click_id(egui::Id::new(("find", "find.3")));
    assert_eq!(
        window.app.arranging.carrying,
        Some(Carry::Thing(DisplayId::find(3)))
    );
    let shelf = window.piece("shelf");
    let at = window.find(&Target::Piece(shelf));
    window.click(at);
    let keeper = window.app.keeper;
    assert_eq!(window.app.state.shown_by(&DisplayId::find(3)), Some(keeper));
    assert!(matches!(
        window.layout().displays.first().map(|shown| shown.spot),
        Some(Spot::On { piece, .. }) if piece == shelf
    ));
    window.click_text("Undo");
    assert_eq!(window.app.state.shown_by(&DisplayId::find(3)), None);
    window.click_text("Redo");
    assert_eq!(window.app.state.shown_by(&DisplayId::find(3)), Some(keeper));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_shell_will_not_stand_on_the_floor_and_stays_in_hand() {
    let data = scratch("floorshell");
    let mut window = Harness::open(&data);
    window.arrange();
    window.click_id(egui::Id::new(("find", "find.3")));
    let at = window.on_window(window.app.scene.view.tile_centre(6, 6));
    window.click(at);
    assert_eq!(window.app.state.shown_by(&DisplayId::find(3)), None);
    assert_eq!(
        window.app.arranging.carrying,
        Some(Carry::Thing(DisplayId::find(3)))
    );
    window.key(Key::Escape);
    assert_eq!(window.app.arranging.carrying, None);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_cushion_dragged_from_the_catalogue_lands_where_it_is_let_go() {
    let data = scratch("cushion");
    let mut window = Harness::open(&data);
    window.arrange();
    window.turn_to(Drawer::Furniture);
    let from = window.at_id(egui::Id::new(("piece", "cushion")));
    let to = window.on_window(window.app.scene.view.tile_centre(6, 3));
    window.drag(from, to);
    let cushion = window
        .layout()
        .pieces
        .iter()
        .find(|placed| placed.piece.as_str() == "cushion")
        .cloned()
        .expect("the cushion is in the room");
    assert_eq!((cushion.x, cushion.y), (6, 3));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_piece_in_the_room_is_picked_up_turned_moved_and_put_away() {
    let data = scratch("move");
    let mut window = Harness::open(&data);
    window.arrange();
    let chair = window.piece("armchair");
    let at = window.find(&Target::Piece(chair));
    window.click(at);
    assert_eq!(
        window.app.arranging.carrying,
        Some(Carry::Piece {
            name: chair,
            turn: 1
        })
    );
    window.key(Key::R);
    let to = window.on_window(window.app.scene.view.tile_centre(6, 6));
    window.step(vec![Event::PointerMoved(to)]);
    window.click(to);
    let placed = window.layout().piece(chair).cloned().unwrap();
    assert_eq!((placed.x, placed.y, placed.turn), (6, 6, 2));
    let at = window.find(&Target::Piece(chair));
    window.click(at);
    window.key(Key::Delete);
    assert!(window.layout().piece(chair).is_none(), "put away");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn what_is_arranged_is_there_when_the_house_is_opened_again() {
    let data = scratch("again");
    {
        let mut window = Harness::open(&data);
        window.arrange();
        window.click_id(egui::Id::new(("find", "find.3")));
        let shelf = window.piece("shelf");
        let at = window.find(&Target::Piece(shelf));
        window.click(at);
        window.turn_to(Drawer::Room);
        window.click_id(egui::Id::new(("floor", "floor.rose")));
        window.click_id(notebook::mode_id(Mode::Live));
        assert_eq!(window.app.mode, Mode::Live);
    }
    let window = Harness::open(&data);
    let layout = window.layout();
    assert_eq!(layout.floor.as_str(), "floor.rose");
    assert_eq!(
        window.app.state.shown_by(&DisplayId::find(3)),
        Some(window.app.keeper)
    );
    let _ = std::fs::remove_dir_all(&data);
}

impl Harness {
    /// Let time pass in big steps: the household's own clock takes a quarter second at a time.
    fn pass(&mut self, seconds: f64) {
        let until = self.time + seconds;
        while self.time < until {
            self.step(Vec::new());
            self.time += 0.25 - 1.0 / 30.0;
        }
    }
}

#[test]
fn a_visitor_who_comes_in_is_listed_and_can_be_chosen() {
    let data = scratch("visitor");
    let mut window = Harness::open(&data);
    assert!(window.text("VISITING").is_none(), "nobody has come yet");
    let visitor = window.app.household.visitors[0].id;
    let name = window.app.household.visitors[0].name.clone();
    window.pass(60.0);
    assert!(
        window.text("VISITING").is_some(),
        "the visitor never came in"
    );
    assert!(
        window.app.notice.is_some() || window.app.life.present().contains(&visitor),
        "the window never said who had come"
    );
    window.click_text(&name);
    assert_eq!(window.app.selected, Some(visitor));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_photo_is_the_whole_room_and_nothing_of_the_window() {
    let data = scratch("photo");
    let mut window = Harness::open(&data);
    let photo = window.app.photo();
    assert_eq!(
        (photo.width(), photo.height()),
        (crate::iso::SCENE_WIDTH, crate::iso::SCENE_HEIGHT)
    );
    assert!(photo.pixels().iter().all(|pixel| pixel.a == 255));
    // The chosen resident's gold ring is the window's, not the room's.
    let keeper = window.keeper();
    assert_eq!(window.app.selected, Some(keeper));
    let (layout, now) = (window.app.house.clone(), window.app.now());
    let overlay = Overlay {
        selected: Some(keeper),
        lamps_off: window.app.life.lamps_off().to_vec(),
        backdrop: true,
        ..Overlay::default()
    };
    let mut marked = window.app.scene.compose(
        &layout,
        &window.app.household.snapshot,
        &mut window.app.life.actors,
        now,
        &overlay,
    );
    assert_ne!(photo, marked, "the photo has no ring");
    marked = window.app.photo();
    assert_eq!(photo, marked, "and the same moment photographs the same");
    let _ = std::fs::remove_dir_all(&data);
}

impl Harness {
    /// Whether the window asked last frame to be closed.
    fn asked_to_close(&self) -> bool {
        self.output.as_ref().is_some_and(|output| {
            output.viewport_output.values().any(|viewport| {
                viewport
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::ViewportCommand::Close))
            })
        })
    }
}

#[test]
fn a_room_chosen_from_the_rooms_page_is_built_where_it_is_shown_and_joins_the_house() {
    let data = scratch("build");
    let mut window = Harness::open(&data);
    window.arrange();
    window.turn_to(Drawer::Room);
    window.click_id(egui::Id::new(("template", "room.nook")));
    let (places, _) = window.app.placing.clone().expect("somewhere to build it");
    assert!(places.len() >= 2);
    // The next place along, then built there.
    window.click_text("\u{25b6}");
    let chosen = window.app.placing.as_ref().unwrap().1;
    assert_eq!(chosen, 1);
    window.click_text("Build here");
    assert!(window.app.placing.is_none());
    let home = home_of(&window.app.state, window.app.keeper).clone();
    assert_eq!(home.rooms, places[1].rooms);
    assert_eq!(window.app.house.reachable(), vec![true, true]);
    // The household lives on through the doorway once arranging is done.
    window.click_id(notebook::mode_id(Mode::Live));
    window.pass(20.0);
    assert!(window.app.life.present().len() >= 2);
    formiga_home_contract::HomeDocument::validate(&window.app.state).unwrap();
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_cover_s_close_stud_leaves_the_house() {
    let data = scratch("close");
    let mut window = Harness::open(&data);
    let close = notebook::stud_id(crate::art::notebook::Glyph::Close);
    let at = window.at_id(close);
    window.step(vec![Event::PointerMoved(at)]);
    window.step(vec![Harness::button(at, PointerButton::Primary, true)]);
    window.step(vec![Harness::button(at, PointerButton::Primary, false)]);
    assert!(window.asked_to_close());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_house_comes_closer_moves_about_under_a_drag_and_fits_again() {
    let data = scratch("zoom");
    let mut window = Harness::open(&data);
    let fitted = window.app.room_rect.unwrap();
    window.key(Key::Plus);
    window.wait(0.1);
    let closer = window.app.room_rect.unwrap();
    assert!(
        closer.width() > fitted.width(),
        "{closer:?} after {fitted:?}"
    );
    assert!(window.app.zoom.closer_than_fits());
    // Dragged across the floor, the house moves, and nothing is asked of anyone.
    let keeper = window.keeper();
    let from = window.find(&Target::Floor(6, 3));
    window.drag(from, from + egui::vec2(-60.0, -40.0));
    window.wait(0.1);
    let moved = window.app.room_rect.unwrap();
    assert_ne!(moved.center(), closer.center());
    assert!(window.app.life.queue(keeper).is_empty());
    assert!(window.app.carried.is_none());
    window.click_id(egui::Id::new(("zoom", "fit")));
    window.wait(0.1);
    assert_eq!(window.app.room_rect.unwrap(), fitted);
    // Never further away than fits.
    window.click_id(egui::Id::new(("zoom", "out")));
    window.wait(0.1);
    assert_eq!(window.app.room_rect.unwrap(), fitted);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_keepsake_left_by_a_friend_is_kept_apart_from_the_finds_and_can_be_let_go() {
    let data = scratch("keepsake");
    let mut window = Harness::open(&data);
    let friend = window.app.household.visitors[0].id;
    let gift = window
        .app
        .make_keepsake(MementoKind::Postcard, Some(friend), Vec::new())
        .unwrap();
    window.arrange();
    window.turn_to(Drawer::Finds);
    window.wait(0.1);
    assert!(
        window
            .text("Left by friends, drawn, or framed, and kept here")
            .is_some(),
        "no page of keepsakes"
    );
    let tile = window.at_id(egui::Id::new(("find", gift.as_str())));
    window.click_with(tile, PointerButton::Secondary);
    window.click_text("Let it go");
    let home = |window: &Harness| {
        window
            .app
            .state
            .household(window.app.keeper)
            .unwrap()
            .clone()
    };
    assert!(home(&window).memento(&gift).is_none());
    assert!(window.app.household.snapshot.item(&gift).is_none());
    window.click_text("Undo");
    assert!(
        home(&window).memento(&gift).is_some(),
        "letting go is undone"
    );
    assert!(window.app.household.snapshot.item(&gift).is_some());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_journal_page_says_who_came_over_today() {
    let data = scratch("journal");
    let mut window = Harness::open(&data);
    let friend = &window.app.household.visitors[0];
    let (id, name) = (friend.id, friend.name.clone());
    window.app.note(HomeMoment::Visit {
        visitor: TravelerId(id),
    });
    window.click_id(notebook::page_id(Drawer::Journal));
    window.wait(0.1);
    assert!(window.text("Today").is_some());
    assert!(window.text(&format!("{name} came over.")).is_some());
    window.click_id(notebook::page_id(Drawer::Household));
    window.wait(0.1);
    assert!(window.text(&format!("{name} came over.")).is_none());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_rehearsal_goes_next_door_in_the_same_window_and_back_again() {
    let data = scratch("next-door");
    let mut window = Harness::open(&data);
    let here = window.app.keeper;
    let village = window.app.household.snapshot.village.clone();
    let name_of = |keeper| {
        village
            .iter()
            .find(|house| house.keeper == keeper)
            .map(|house| format!("{}'s house", house.name))
            .unwrap()
    };
    let next = village
        .iter()
        .find(|house| house.keeper != here)
        .unwrap()
        .keeper;
    window.click_text(&name_of(next));
    window.wait(0.2);
    assert_eq!(window.app.keeper, next);
    assert_eq!(window.app.household.keeper().id, next.0);
    assert!(
        window.app.state.household(next).is_some(),
        "its house is set up"
    );
    window.click_text(&name_of(here));
    window.wait(0.2);
    assert_eq!(window.app.keeper, here, "and back home again");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_close_friend_can_be_asked_to_stay_over_and_to_move_in_from_its_menu() {
    let data = scratch("move-in");
    let mut window = Harness::open(&data);
    let friend = window.app.household.visitors[0].id;
    while !window.app.life.present().contains(&friend) {
        window.wait(1.0);
        assert!(window.app.now() < 90.0, "the friend never came");
    }
    window.app.selected = Some(friend);
    let at = window.find(&Target::Resident(friend));
    window.click(at);
    let name = window.app.name(friend);
    window.click_text(&format!("Ask {name} to stay over"));
    assert!(window.app.life.staying(friend));
    let at = window.find(&Target::Resident(friend));
    window.click(at);
    window.click_text(&format!("Ask {name} to move in"));
    assert_eq!(window.app.move_in, Some(friend));
    assert_eq!(
        window.app.lived().move_in,
        Some((TravelerId(friend), window.app.keeper))
    );
    assert!(window.app.move_in_entry(friend).is_none(), "once a visit");
    let journal = &window
        .app
        .state
        .household(window.app.keeper)
        .unwrap()
        .journal;
    assert!(matches!(
        journal.last().map(|entry| &entry.moment),
        Some(HomeMoment::AskedToMoveIn { visitor }) if visitor.0 == friend
    ));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_notebook_answers_to_its_keys() {
    let data = scratch("keys");
    let mut window = Harness::open(&data);
    window.app.selected = None;
    window.key(Key::N);
    let first = window.app.household.residents[0].id;
    assert_eq!(window.app.selected, Some(first), "N chooses the first one");
    window.key(Key::N);
    assert_ne!(window.app.selected, Some(first), "and then the next");
    window.key(Key::CloseBracket);
    assert_eq!(window.app.live_page, Drawer::Journal);
    window.key(Key::OpenBracket);
    assert_eq!(window.app.live_page, Drawer::Household);
    window.key(Key::H);
    assert!(window.app.help);
    window.key(Key::H);
    assert!(!window.app.help);
    window.key(Key::A);
    assert_eq!(window.app.mode, Mode::Arrange);
    window.key(Key::CloseBracket);
    assert_eq!(window.app.drawer, Drawer::Furniture);
    window.key(Key::L);
    assert_eq!(window.app.mode, Mode::Live);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn dragging_the_leather_moves_the_window_and_a_double_click_opens_it_wide() {
    let data = scratch("leather");
    let mut window = Harness::open(&data);
    // The cover's left edge, below its studs and clear of the page.
    let on_leather = Pos2::new(8.0, 400.0);
    window.drag(on_leather, on_leather + egui::vec2(80.0, 30.0));
    assert!(
        window
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::StartDrag)),
        "{:?}",
        window.commands
    );
    window.commands.clear();
    window.click(on_leather);
    window.click(on_leather);
    assert!(
        window
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Maximized(true)))
    );
    let _ = std::fs::remove_dir_all(&data);
}
