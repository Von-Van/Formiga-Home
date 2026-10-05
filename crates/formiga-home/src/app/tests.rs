//! The window driven as a person would drive it: frames run headless through egui with pointer
//! and key events, things in the room found by asking the scene what is under the pointer, and
//! what each click and drag did read back from the household and the homes.

use super::*;
use crate::host::Rehearsal;
use crate::store::RehearsalHomes;
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use formiga_home_contract::{DisplayId, sample};
use std::path::Path;

struct Harness {
    ctx: egui::Context,
    app: HomeApp,
    time: f64,
    output: Option<egui::FullOutput>,
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
        let host = Host::Rehearsal(Rehearsal::new(snapshot, homes, "test".to_owned()));
        let ctx = egui::Context::default();
        let app = HomeApp::new(&ctx, household, host, Some(data.to_owned()), None);
        let mut harness = Self {
            ctx,
            app,
            time: 0.0,
            output: None,
        };
        harness.wait(0.2);
        harness
    }

    fn step(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 640.0))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame(ui));
        // No renderer here to hand the textures to.
        output.textures_delta.clear();
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

    fn click_text(&mut self, wanted: &str) {
        let at = self
            .text(wanted)
            .unwrap_or_else(|| panic!("nothing says {wanted:?}"));
        self.click(at);
    }

    /// A point of the scene, on the window.
    fn on_window(&self, (x, y): (f32, f32)) -> Pos2 {
        let rect = self.app.room_rect.expect("the room has been drawn");
        let scale = rect.width() / SCENE_WIDTH as f32;
        rect.min + egui::vec2(x + 0.5, y + 0.5) * scale
    }

    /// Somewhere on the window the room shows `target` under the pointer.
    fn find(&mut self, target: &Target) -> Pos2 {
        let view = self.app.scene.view;
        let layout = layout_of(&self.app.state, self.app.keeper).clone();
        let (cx, cy) = match target {
            Target::Floor(x, y) => view.tile_centre(*x, *y),
            Target::Piece(uid) => {
                let footprint = room::footprint(layout.piece(*uid).unwrap());
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
                let spot = layout
                    .displays
                    .iter()
                    .find(|s| &s.item == item)
                    .unwrap()
                    .spot;
                let (x, y) = self.app.scene.spot_anchor(&layout, spot).unwrap();
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
        layout_of(&self.app.state, self.app.keeper)
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

    fn layout(&self) -> RoomLayout {
        layout_of(&self.app.state, self.app.keeper).clone()
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
    let layout = window.layout();
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
    let layout = window.layout();
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
    window.click_text("Arrange");
    assert_eq!(window.app.mode, Mode::Arrange);
    window.click_text("Shell");
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
    window.click_text("Arrange");
    window.click_text("Shell");
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
    window.click_text("Arrange");
    window.click_text("Furniture");
    let from = window
        .text("Floor cushion")
        .expect("the cushion is in the catalogue");
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
    window.click_text("Arrange");
    let chair = window.piece("armchair");
    let at = window.find(&Target::Piece(chair));
    window.click(at);
    assert_eq!(
        window.app.arranging.carrying,
        Some(Carry::Piece {
            uid: chair,
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
        window.click_text("Arrange");
        window.click_text("Shell");
        let shelf = window.piece("shelf");
        let at = window.find(&Target::Piece(shelf));
        window.click(at);
        window.click_text("Room");
        window.click_text("Rose carpet");
        window.click_text("Live");
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
