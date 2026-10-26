//! Every room Home draws, checked pixel for pixel against how it was drawn before: houses of one
//! room and of three, by day, at dusk and by night with a lamp on and off, in winter, with someone
//! pointed at and chosen, while arranging, with a piece from a newer Home among the furniture, and
//! a household living its own life in them for a while. What the pointer finds at each point is
//! checked too, and so is every step of that life. A change meant only to make drawing faster
//! must leave all of them exactly as they were.
//!
//! Each picture is kept as a hash of its pixels, in the `golden` folder beside this file. When a
//! picture is changed on purpose, run the tests with `FORMIGA_HOME_BLESS=1` to write the new
//! hashes, and say in the commit which pictures changed and why.
//!
//! The hashes are only checked on macOS, where they were taken: the light and the residents'
//! walks are worked out with square roots and sines, and another system's maths library may round
//! one of those a hair differently and move a pixel. Everywhere else the pictures are still drawn,
//! so nothing panics.

use super::*;
use crate::household::Household;
use crate::life::{Act, Life};
use crate::{arrange, keepsakes, show_house, starter};
use formiga_home_contract::{CatalogId, HouseholdHome, PlacedPiece, sample};
use std::fmt::Write as _;
use std::path::Path;

/// FNV-1a, over whatever is fed to it.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn eat(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// A picture's pixels as one number: its size and every channel of every pixel.
fn hash(canvas: &Canvas) -> u64 {
    let mut hash = Fnv::new();
    hash.eat(&canvas.width().to_le_bytes());
    hash.eat(&canvas.height().to_le_bytes());
    for pixel in canvas.pixels() {
        hash.eat(&[pixel.r, pixel.g, pixel.b, pixel.a]);
    }
    hash.0
}

/// The pictures of one kind, by name.
struct Pictures(Vec<(String, u64)>);

impl Pictures {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, name: impl Into<String>, canvas: &Canvas) {
        self.0.push((name.into(), hash(canvas)));
    }

    fn add_hash(&mut self, name: impl Into<String>, hash: u64) {
        self.0.push((name.into(), hash));
    }

    /// Checks these pictures against the hashes kept in `golden/<kind>.txt`, saying which have
    /// changed, which are new and which have gone; or writes them there, if asked to.
    fn check(self, kind: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/scene/golden")
            .join(format!("{kind}.txt"));
        let mut table = String::new();
        for (name, hash) in &self.0 {
            writeln!(table, "{name} {hash:016x}").unwrap();
        }
        if std::env::var_os("FORMIGA_HOME_BLESS").is_some_and(|value| !value.is_empty()) {
            std::fs::write(&path, &table).unwrap();
            return;
        }
        if !cfg!(target_os = "macos") {
            return;
        }
        let kept = std::fs::read_to_string(&path).unwrap_or_default();
        let kept: Vec<(&str, &str)> = kept
            .lines()
            .filter_map(|line| line.trim().split_once(' '))
            .collect();
        let mut differences = Vec::new();
        for (name, hash) in &self.0 {
            let now = format!("{hash:016x}");
            match kept.iter().find(|(kept_name, _)| kept_name == name) {
                Some((_, before)) if *before == now => {}
                Some((_, before)) => {
                    differences.push(format!("  changed  {name}: {before} -> {now}"));
                }
                None => differences.push(format!("  new      {name}: {now}")),
            }
        }
        for (name, before) in &kept {
            if !self.0.iter().any(|(now, _)| now == name) {
                differences.push(format!("  gone     {name}: {before}"));
            }
        }
        assert!(
            differences.is_empty(),
            "{} of the {kind} pictures are not drawn as they were:\n{}\n\n\
             If that was meant, run the tests with FORMIGA_HOME_BLESS=1 to keep the new hashes.",
            differences.len(),
            differences.join("\n"),
        );
    }
}

/// A house to draw: its rooms, the snapshot stocked with its keepsakes, and the house they make.
struct Setting {
    home: HouseholdHome,
    snapshot: HomeSnapshot,
    house: House,
}

/// The sample house as it first opens, or lived in; grown to `rooms` rooms; and with a piece
/// only a newer Home knows, if `stranger`.
fn setting(household: &Household, lived_in: bool, rooms: usize, stranger: bool) -> Setting {
    let home = if lived_in {
        show_house::lived_in(household, "floor.checks", "wall.stripes")
    } else {
        starter::home(&household.snapshot)
    };
    let mut home = show_house::grown(home, rooms, &household.snapshot);
    arrange::settle(&mut home);
    if stranger {
        let room = &mut home.rooms[0];
        let uid = room
            .pieces
            .iter()
            .map(|placed| placed.uid)
            .max()
            .unwrap_or(0)
            + 1;
        room.pieces.push(PlacedPiece {
            uid,
            piece: CatalogId::parse("hammock_of_the_future").unwrap(),
            x: 1,
            y: 1,
            turn: 0,
        });
    }
    let mut snapshot = household.snapshot.clone();
    keepsakes::stock(&mut snapshot, &home);
    let house = House::of(&home.rooms);
    Setting {
        home,
        snapshot,
        house,
    }
}

fn hour(hour: f32, month: time::Month) -> Daylight {
    Daylight { hour, month }
}

/// What the pointer finds all over the picture, as one number.
fn hits(
    scene: &mut Scene,
    house: &House,
    snapshot: &HomeSnapshot,
    actors: &mut [Actor],
    now: f32,
) -> u64 {
    let (width, height) = (scene.view.size.0 as i32, scene.view.size.1 as i32);
    let mut found = Fnv::new();
    for y in (0..height).step_by(9) {
        for x in (0..width).step_by(9) {
            let target = scene.hit(house, snapshot, actors, now, (x, y));
            found.eat(format!("{target:?};").as_bytes());
        }
    }
    found.0
}

/// One house, still, every way it can be drawn, one frame after another on the same scene.
fn still_frames(pictures: &mut Pictures, household: &Household, name: &str, setting: &Setting) {
    let Setting {
        home,
        snapshot,
        house,
    } = setting;
    let mut actors = show_house::pose(household, house, household.reduce_motion());
    let mut scene = Scene::new(house);
    scene.set_pictures(keepsakes::pictures(home));
    let lamp = house
        .pieces
        .iter()
        .find(|placed| catalog::piece(&placed.piece).is_some_and(|piece| piece.glow().is_some()))
        .map(|placed| placed.uid);
    let standing = house
        .pieces
        .iter()
        .find(|placed| catalog::piece(&placed.piece).is_some_and(|piece| !piece.flat))
        .map(|placed| placed.uid);
    let resident = actors.get(1).map(|actor| actor.id);
    let chosen = actors.first().map(|actor| actor.id);
    let night = hour(22.0, time::Month::June);
    let frame = |pictures: &mut Pictures,
                 scene: &mut Scene,
                 actors: &mut [Actor],
                 now: f32,
                 what: &str,
                 overlay: Overlay| {
        let canvas = scene.compose(house, snapshot, actors, now, &overlay);
        pictures.add(format!("{name}.{what}"), &canvas);
    };
    for (what, daylight) in [
        ("noon", Daylight::default()),
        ("dusk", hour(19.4, time::Month::June)),
        ("night", night),
        ("winter_morning", hour(7.0, time::Month::December)),
    ] {
        let overlay = Overlay {
            daylight,
            ..Overlay::default()
        };
        frame(pictures, &mut scene, &mut actors, 0.5, what, overlay);
    }
    let overlay = Overlay {
        daylight: night,
        lamps_off: lamp.into_iter().collect(),
        ..Overlay::default()
    };
    frame(
        pictures,
        &mut scene,
        &mut actors,
        0.5,
        "night.lamp_off",
        overlay,
    );
    let overlay = Overlay {
        daylight: night,
        hovered: resident.map(Target::Resident),
        selected: chosen,
        ..Overlay::default()
    };
    frame(
        pictures,
        &mut scene,
        &mut actors,
        0.5,
        "night.pointed_at",
        overlay,
    );
    // The same moment again with the pointer moved on: only what is pointed at changes.
    let overlay = Overlay {
        daylight: night,
        hovered: Some(Target::Floor(2, 2)),
        selected: chosen,
        ..Overlay::default()
    };
    frame(
        pictures,
        &mut scene,
        &mut actors,
        0.5,
        "night.floor",
        overlay,
    );
    let overlay = Overlay {
        hovered: standing.map(Target::Piece),
        ..Overlay::default()
    };
    frame(pictures, &mut scene, &mut actors, 0.5, "piece", overlay);
    let overlay = Overlay {
        hovered: house
            .shown
            .first()
            .map(|shown| Target::Shown(shown.item.clone())),
        ..Overlay::default()
    };
    frame(pictures, &mut scene, &mut actors, 0.5, "shown", overlay);
    // A moment later, as the residents breathe and blink.
    for now in [1.3, 2.9, 4.05] {
        let overlay = Overlay {
            hovered: resident.map(Target::Resident),
            ..Overlay::default()
        };
        frame(
            pictures,
            &mut scene,
            &mut actors,
            now,
            &format!("at_{now}"),
            overlay,
        );
    }
    // Arranging: a piece lifted and carried, outlined where it would go.
    if let Some(uid) = standing {
        let placed = house.piece(uid).unwrap();
        let kind = catalog::piece(&placed.piece).unwrap();
        let ghost = Ghost {
            sprite: scene.piece_sprite(kind, placed.turn + 1).clone(),
            at: scene.view.pixel(2.0, 3.0),
            fits: false,
            footprint: Some(Footprint {
                x: 2,
                y: 3,
                w: kind.size.0,
                d: kind.size.1,
            }),
        };
        let overlay = Overlay {
            arranging: true,
            lifted: Some(Target::Piece(uid)),
            ghost: Some(ghost),
            ..Overlay::default()
        };
        frame(pictures, &mut scene, &mut actors, 0.5, "arranging", overlay);
    }
    let overlay = Overlay {
        arranging: true,
        chosen_room: Some(0),
        new_room: house.rooms.len().checked_sub(1).map(|room| room as u8),
        door: house.walls.first().map(|wall| (*wall, true)),
        ..Overlay::default()
    };
    frame(pictures, &mut scene, &mut actors, 0.5, "rooms", overlay);
    let overlay = Overlay {
        backdrop: true,
        daylight: hour(20.5, time::Month::October),
        ..Overlay::default()
    };
    frame(pictures, &mut scene, &mut actors, 0.5, "photo", overlay);
    // And by day once more, after the photo, as the window goes back to drawing itself.
    frame(
        pictures,
        &mut scene,
        &mut actors,
        0.5,
        "after_photo",
        Overlay::default(),
    );
    let found = hits(&mut scene, house, snapshot, &mut actors, 0.5);
    pictures.add_hash(format!("{name}.hits"), found);
}

#[test]
fn every_room_is_drawn_as_it_was() {
    let household = Household::new(sample::snapshot()).unwrap();
    let mut pictures = Pictures::new();
    for (name, lived_in, rooms, stranger) in [
        ("starter", false, 1, false),
        ("lived_in", true, 1, false),
        ("two_rooms", true, 2, false),
        ("three_rooms", true, 3, true),
        ("starter.three_rooms", false, 3, false),
    ] {
        let setting = setting(&household, lived_in, rooms, stranger);
        still_frames(&mut pictures, &household, name, &setting);
    }
    pictures.check("rooms");
}

/// Forty seconds of the household's own life in a house, with a pat, a lift or an errand now and
/// then: every step of it, and a picture every second and a half, as the window would draw it.
fn a_while(pictures: &mut Pictures, household: &Household, name: &str, daylight: Daylight) {
    let Setting {
        home,
        snapshot,
        house,
    } = setting(household, true, 3, true);
    let mut scene = Scene::new(&house);
    scene.set_pictures(keepsakes::pictures(&home));
    let mut life = Life::new(household, &house);
    life.set_dark(daylight.dark().0);
    let mut steps = Fnv::new();
    let mut dice = 2024_u64;
    let mut carried: Option<(Id, f32)> = None;
    let dt = 1.0 / 30.0;
    let mut now = 0.0;
    for tick in 0..1200_u32 {
        if tick % 89 == 0 && carried.is_none() {
            dice = dice
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let present = life.present();
            if !present.is_empty() {
                let who = present[(dice >> 33) as usize % present.len()];
                match (dice >> 20) % 4 {
                    0 => life.pet(household, who, now),
                    1 => {
                        life.pick_up(who, now);
                        carried = Some((who, now));
                    }
                    2 => {
                        let (x, y) = (((dice >> 40) % 8) as u8, ((dice >> 44) % 8) as u8);
                        life.ask(who, Act::GoTo(x, y), now);
                    }
                    _ => {}
                }
            }
        }
        if let Some((who, since)) = carried
            && now - since > 1.0
        {
            life.put_down(&house, who, now);
            carried = None;
        }
        now += dt;
        life.tick(household, &house, &snapshot, &home.likings, now, dt);
        for actor in &life.actors {
            steps.eat(
                format!(
                    "{}:{:?}:{}:{:?}:{:?}:{}:{:?};",
                    actor.id,
                    actor.pos,
                    actor.facing_right,
                    actor.lift,
                    actor.on_piece,
                    actor.hidden,
                    actor.pose,
                )
                .as_bytes(),
            );
        }
        steps.eat(format!("{:?}{:?}", life.lamps_off(), life.worn()).as_bytes());
        if tick % 45 == 44 {
            let mut seen = house.clone();
            let worn = life.worn();
            seen.shown.retain(|shown| !worn.contains(&shown.item));
            let present = life.present();
            let hovered = match (tick / 45) % 4 {
                0 => present.first().copied().map(Target::Resident),
                1 => present.last().copied().map(Target::Resident),
                2 => seen.pieces.first().map(|placed| Target::Piece(placed.uid)),
                _ => None,
            };
            let overlay = Overlay {
                hovered,
                selected: present.first().copied(),
                lamps_off: life.lamps_off().to_vec(),
                daylight,
                ..Overlay::default()
            };
            let canvas = scene.compose(&seen, &snapshot, &mut life.actors, now, &overlay);
            pictures.add(format!("{name}.{:02}", tick / 45), &canvas);
            if tick % 600 == 599 {
                let found = hits(&mut scene, &seen, &snapshot, &mut life.actors, now);
                pictures.add_hash(format!("{name}.{:02}.hits", tick / 45), found);
            }
        }
    }
    pictures.add_hash(format!("{name}.life"), steps.0);
}

#[test]
fn a_household_lives_and_is_drawn_as_it_was() {
    let household = Household::new(sample::snapshot()).unwrap();
    let mut pictures = Pictures::new();
    a_while(&mut pictures, &household, "day", Daylight::default());
    a_while(
        &mut pictures,
        &household,
        "night",
        hour(23.0, time::Month::June),
    );
    pictures.check("lives");
}
