use super::*;
use crate::household::Household;
use crate::show_house;
use formiga_home_contract::{CatalogId, HomeMoment, Liked, Liking, sample};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "formiga-home-designs-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The sample household's homes, its house lived in for a while.
fn lived_in() -> (HomeSnapshot, HomeState) {
    let snapshot = sample::snapshot();
    let household = Household::new(snapshot.clone()).unwrap();
    let mut state = HomeState::new(&snapshot.colony_key);
    state.set_household(show_house::lived_in(
        &household,
        "floor.rose",
        "wall.plaster",
    ));
    (snapshot, state)
}

fn keeper(snapshot: &HomeSnapshot) -> TravelerId {
    snapshot.household.keeper
}

fn at() -> OffsetDateTime {
    time::macros::datetime!(2026-10-15 12:00 UTC)
}

fn room(width: u8, depth: u8) -> RoomLayout {
    RoomLayout {
        width,
        depth,
        floor: CatalogId::known("floor.boards"),
        wall: CatalogId::known("wall.plaster"),
        pieces: Vec::new(),
        displays: Vec::new(),
        plan: None,
        kind: None,
        doors: Vec::new(),
    }
}

fn piece(uid: u16, id: &'static str, x: u8, y: u8) -> PlacedPiece {
    PlacedPiece {
        uid,
        piece: CatalogId::known(id),
        x,
        y,
        turn: 0,
    }
}

fn one_room(layout: RoomLayout) -> Design {
    Design {
        format: FORMAT.to_owned(),
        version: VERSION,
        min_reader_version: 1,
        name: "Test".to_owned(),
        whole_house: false,
        saved_at_utc: at(),
        rooms: vec![layout],
    }
}

#[test]
fn a_house_used_again_as_it_was_saved_is_exactly_as_it_was() {
    let (snapshot, state) = lived_in();
    let home = state.household(keeper(&snapshot)).unwrap();
    let design = Design::of(home, Scope::House, "Autumn", at());
    let fitted = fit(&design, &state, keeper(&snapshot), &snapshot, 0).unwrap();
    // Only the keepsakes, which a design never takes, go back in the drawer.
    let mut expected = home.clone();
    for room in &mut expected.rooms {
        room.displays
            .retain(|shown| shown.item.source() != DisplayId::MEMENTO);
    }
    assert_eq!(fitted.home, expected);
    assert!(fitted.omitted.is_empty(), "{:?}", fitted.omitted);
    assert!(
        fitted
            .put_away
            .iter()
            .all(|item| item.source() == DisplayId::MEMENTO)
    );
}

#[test]
fn a_room_put_back_after_it_was_cleared_comes_back_where_everything_was() {
    let (snapshot, mut state) = lived_in();
    let keeper = keeper(&snapshot);
    let design = Design::of(
        state.household(keeper).unwrap(),
        Scope::Room(0),
        "Cosy",
        at(),
    );
    let saved = state.household(keeper).unwrap().rooms[0].clone();
    let home = state.household_mut(keeper).unwrap();
    home.rooms[0].pieces.clear();
    home.rooms[0].displays.clear();
    home.rooms[0].floor = CatalogId::known("floor.boards");
    let fitted = fit(&design, &state, keeper, &snapshot, 0).unwrap();
    let room = &fitted.home.rooms[0];
    assert_eq!(room.floor, saved.floor);
    let at = |layout: &RoomLayout| {
        let mut pieces: Vec<_> = layout
            .pieces
            .iter()
            .map(|placed| (placed.piece.clone(), placed.x, placed.y, placed.turn))
            .collect();
        pieces.sort();
        pieces
    };
    assert_eq!(at(room), at(&saved));
    let finds = |layout: &RoomLayout| {
        layout
            .displays
            .iter()
            .filter(|shown| shown.item.source() != DisplayId::MEMENTO)
            .count()
    };
    assert_eq!(room.displays.len(), finds(&saved));
}

#[test]
fn using_a_design_keeps_the_households_history_and_what_it_liked() {
    let (snapshot, mut state) = lived_in();
    let keeper = keeper(&snapshot);
    let design = Design::of(state.household(keeper).unwrap(), Scope::House, "Kept", at());
    let home = state.household_mut(keeper).unwrap();
    let sofa = home.rooms[0]
        .pieces
        .iter()
        .find(|placed| placed.piece.as_str() == "sofa")
        .unwrap()
        .uid;
    let resident = snapshot.residents[0].id;
    home.likings.push(Liking {
        resident,
        thing: Liked::Piece { room: 0, uid: sofa },
        uses: 3,
    });
    home.note(
        at(),
        HomeMoment::Room {
            room: CatalogId::known("room.nook"),
        },
    );
    // The sofa has been moved since the design was saved.
    home.rooms[0]
        .pieces
        .iter_mut()
        .find(|placed| placed.uid == sofa)
        .unwrap()
        .y = 0;
    let before = home.clone();
    let fitted = fit(&design, &state, keeper, &snapshot, 0).unwrap();
    assert_eq!(fitted.home.keeper, before.keeper);
    assert_eq!(fitted.home.likings, before.likings);
    assert_eq!(fitted.home.journal, before.journal);
    assert_eq!(fitted.home.mementos, before.mementos);
    assert_eq!(fitted.home.projects, before.projects);
    assert_eq!(fitted.home.stays_out, before.stays_out);
    let back = fitted.home.rooms[0].piece(sofa).unwrap();
    assert_ne!(
        back.y, 0,
        "the sofa is back where the design had it, under its own name"
    );
}

#[test]
fn what_has_not_arrived_yet_is_left_out_and_said_why() {
    let (mut snapshot, state) = lived_in();
    let design = Design::of(
        state.household(keeper(&snapshot)).unwrap(),
        Scope::House,
        "Later",
        at(),
    );
    // A household that has only just moved in: no sofa, no long rug.
    snapshot.days_lived = 0;
    let fitted = fit(&design, &state, keeper(&snapshot), &snapshot, 0).unwrap();
    let not_yet: Vec<&str> = fitted
        .omitted
        .iter()
        .filter_map(|omitted| match omitted {
            Omitted::Piece(Some(piece), Why::NotYet) => Some(piece.id),
            _ => None,
        })
        .collect();
    assert!(
        not_yet.contains(&"sofa") && not_yet.contains(&"long_rug"),
        "{not_yet:?}"
    );
    assert!(fitted.home.rooms[0].pieces.iter().all(|placed| {
        catalog::piece(&placed.piece).unwrap().available(&Arrived {
            days: 0,
            ..Arrived::of(&snapshot, &fitted.home)
        })
    }));
}

#[test]
fn what_the_colony_does_not_have_is_left_out_and_nothing_is_made_up() {
    let (snapshot, state) = lived_in();
    let mut layout = room(6, 6);
    layout.displays.push(PlacedDisplay {
        item: DisplayId::souvenir("not_a_souvenir_anyone_has").unwrap(),
        spot: Spot::Wall {
            side: WallSide::North,
            at: 2,
        },
    });
    let fitted = fit(&one_room(layout), &state, keeper(&snapshot), &snapshot, 0).unwrap();
    assert!(matches!(
        fitted.omitted.as_slice(),
        [Omitted::Thing(_, Why::NotHad)]
    ));
    assert!(fitted.home.rooms[0].displays.is_empty());
}

#[test]
fn a_piece_this_home_does_not_know_is_left_out_not_kept() {
    let (snapshot, state) = lived_in();
    let mut layout = room(6, 6);
    layout.pieces.push(PlacedPiece {
        uid: 1,
        piece: CatalogId::parse("hammock_of_the_future").unwrap(),
        x: 1,
        y: 1,
        turn: 0,
    });
    let fitted = fit(&one_room(layout), &state, keeper(&snapshot), &snapshot, 0).unwrap();
    assert_eq!(fitted.omitted, vec![Omitted::Piece(None, Why::Unknown)]);
    assert!(fitted.home.rooms[0].pieces.is_empty());
}

#[test]
fn a_bigger_room_fits_into_a_smaller_one_as_far_as_it_can() {
    let (snapshot, mut state) = lived_in();
    let keeper = keeper(&snapshot);
    let small = state.household(keeper).unwrap().rooms[0].clone();
    let (width, depth) = (small.width, small.depth);
    let home = state.household_mut(keeper).unwrap();
    home.rooms[0].pieces.clear();
    home.rooms[0].displays.clear();
    let mut big = room(16, 16);
    // A bed in the far corner of a big room, and more armchairs than the small room holds.
    big.pieces.push(piece(1, "bed", 15, 14));
    let mut uid = 2;
    for y in 0..16 {
        for x in 0..16 {
            if (x, y) != (15, 14) && (x, y) != (15, 15) {
                big.pieces.push(piece(uid, "armchair", x, y));
                uid += 1;
            }
        }
    }
    let mut design = one_room(big);
    // More than the room has tiles for, and fewer than a house may hold.
    design.rooms[0]
        .pieces
        .truncate((usize::from(width) * usize::from(depth) + 5).min(90));
    let fitted = fit(&design, &state, keeper, &snapshot, 0).unwrap();
    let layout = &fitted.home.rooms[0];
    assert_eq!(
        (layout.width, layout.depth),
        (width, depth),
        "the room keeps its size"
    );
    let bed = layout
        .pieces
        .iter()
        .find(|placed| placed.piece.as_str() == "bed")
        .expect("the bed comes, as near the corner as it fits");
    assert!(bed.x + 1 >= width - 1 && bed.y >= depth - 3, "{bed:?}");
    assert!(
        fitted
            .omitted
            .iter()
            .all(|omitted| matches!(omitted, Omitted::Piece(Some(_), Why::NoSpace)))
    );
    assert!(!fitted.omitted.is_empty());
    // And what came makes a sound home, with nothing standing in a doorway.
    let mut whole = state.clone();
    whole.set_household(fitted.home.clone());
    whole.validate().unwrap();
    let house = House::of(&fitted.home.rooms);
    for placed in &layout.pieces {
        let piece = catalog::piece(&placed.piece).unwrap();
        assert!(
            piece.flat
                || !crate::arrange::blocks_a_doorway(
                    &house,
                    crate::arrange::footprint_at(&house, 0, piece, placed.x, placed.y, placed.turn)
                )
        );
    }
}

#[test]
fn a_find_shown_in_another_house_is_moved_here_never_copied() {
    let (snapshot, mut state) = lived_in();
    let keeper = keeper(&snapshot);
    let design = Design::of(
        state.household(keeper).unwrap(),
        Scope::House,
        "Moved",
        at(),
    );
    let shell = DisplayId::find(3);
    // The shell went next door since.
    let mut next_door = state.household(keeper).unwrap().clone();
    next_door.keeper = TravelerId(keeper.0 + 1000);
    for layout in &mut next_door.rooms {
        layout.displays.retain(|shown| shown.item == shell);
    }
    next_door.mementos.clear();
    state.household_mut(keeper).unwrap().take_down(&shell);
    state.set_household(next_door.clone());
    let fitted = fit(&design, &state, keeper, &snapshot, 0).unwrap();
    assert_eq!(fitted.moved, vec![(shell, next_door.keeper)]);
}

#[test]
fn a_design_carries_no_keepsake_and_nobody_who_lives_in_the_house() {
    let (snapshot, state) = lived_in();
    let home = state.household(keeper(&snapshot)).unwrap();
    assert!(home.shown().any(|item| item.source() == DisplayId::MEMENTO));
    let design = Design::of(home, Scope::House, "Shared", at());
    let text = String::from_utf8(encode(&design).unwrap()).unwrap();
    assert!(!text.contains(DisplayId::MEMENTO));
    assert!(!text.contains(&snapshot.colony_key));
    for resident in &snapshot.residents {
        assert!(!text.contains(&resident.id.0.to_string()), "{text}");
        assert!(!text.contains(&resident.name));
    }
}

#[test]
fn a_design_file_reads_back_as_it_was_written() {
    let (snapshot, state) = lived_in();
    let home = state.household(keeper(&snapshot)).unwrap();
    let design = Design::of(home, Scope::Room(0), "  Front\troom\u{7}  ", at());
    assert_eq!(design.name, "Front room");
    assert_eq!(decode(&encode(&design).unwrap()).unwrap(), design);
}

#[test]
fn a_file_that_is_not_a_sound_design_is_refused_with_why() {
    let design = one_room(room(6, 6));
    let written = |change: &dyn Fn(&mut serde_json::Value)| {
        let mut value = serde_json::to_value(&design).unwrap();
        change(&mut value);
        decode(&serde_json::to_vec(&value).unwrap()).unwrap_err().0
    };
    assert_eq!(decode(b"{ not json").unwrap_err().0, NOT_A_DESIGN);
    assert_eq!(
        written(&|value| value["format"] = "formiga.hill.share".into()),
        NOT_A_DESIGN
    );
    assert!(written(&|value| value["min_reader_version"] = 2.into()).contains("newer"));
    assert!(written(&|value| value["name"] = "\u{202e}evil".into()).contains("name"));
    assert!(written(&|value| value["rooms"][0]["width"] = 40.into()).contains("rooms"));
    assert!(
        written(&|value| {
            value["rooms"][0]["displays"] = serde_json::json!([
                { "item": "memento.7-1", "spot": { "kind": "floor", "x": 1, "y": 1 } }
            ]);
        })
        .contains("rooms")
    );
    assert!(
        written(&|value| {
            let first = value["rooms"][0].clone();
            value["rooms"] = serde_json::json!([first.clone(), first]);
        })
        .contains("more rooms")
    );
    let huge = vec![b' '; MAX_DESIGN_BYTES as usize + 1];
    assert!(decode(&huge).unwrap_err().0.contains("larger"));
}

#[test]
fn the_gallery_keeps_its_designs_until_one_is_removed() {
    let data = scratch("gallery");
    let (snapshot, state) = lived_in();
    let home = state.household(keeper(&snapshot)).unwrap();
    let mut gallery = Gallery::open(Some(&data));
    assert!(gallery.kept.is_empty());
    gallery
        .keep(Design::of(home, Scope::House, "One", at()), false)
        .unwrap();
    gallery
        .keep(Design::of(home, Scope::Room(0), "Two", at()), true)
        .unwrap();
    // Something that is not a design in the gallery's folder is passed over, and left alone.
    std::fs::write(data.join("designs/stray.json"), b"{}").unwrap();
    let mut again = Gallery::open(Some(&data));
    let names: Vec<(&str, bool)> = again
        .kept
        .iter()
        .map(|kept| (kept.design.name.as_str(), kept.opened))
        .collect();
    assert_eq!(names, vec![("Two", true), ("One", false)]);
    again.remove(0).unwrap();
    let last = Gallery::open(Some(&data));
    assert_eq!(last.kept.len(), 1);
    assert_eq!(last.kept[0].design.name, "One");
    assert!(data.join("designs/stray.json").exists());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_full_gallery_says_so_rather_than_letting_one_go() {
    let (snapshot, state) = lived_in();
    let home = state.household(keeper(&snapshot)).unwrap();
    let mut gallery = Gallery::open(None);
    for n in 0..MAX_DESIGNS {
        gallery
            .keep(
                Design::of(home, Scope::Room(0), &format!("Design {n}"), at()),
                false,
            )
            .unwrap();
    }
    let refused = gallery.keep(Design::of(home, Scope::Room(0), "One more", at()), false);
    assert!(refused.unwrap_err().0.contains("full"));
    assert_eq!(gallery.kept.len(), MAX_DESIGNS);
}

#[test]
fn a_design_exported_opens_as_the_same_design() {
    let data = scratch("export");
    std::fs::create_dir_all(&data).unwrap();
    let (snapshot, state) = lived_in();
    let design = Design::of(
        state.household(keeper(&snapshot)).unwrap(),
        Scope::House,
        "Sunny / rooms?",
        at(),
    );
    assert_eq!(file_name(&design), format!("Sunny rooms.{EXTENSION}"));
    let path = data.join(file_name(&design));
    export(&design, &path).unwrap();
    assert_eq!(read(&path).unwrap(), design);
    assert_eq!(read(&data).unwrap_err().0, NOT_A_DESIGN);
    let _ = std::fs::remove_dir_all(&data);
}
