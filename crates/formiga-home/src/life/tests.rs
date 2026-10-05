use super::*;
use crate::staging;
use crate::starter;
use formiga_home_contract::sample;

fn home() -> (Household, House) {
    let household = Household::new(sample::snapshot()).unwrap();
    let layout = House::of(&staging::lived_in(&household, "floor.boards", "wall.leafy").rooms);
    (household, layout)
}

/// The sample household with every resident of one temperament.
fn tempered(kind: TemperamentKind) -> (Household, House) {
    let mut snapshot = sample::snapshot();
    for resident in &mut snapshot.residents {
        resident.character.temperament = kind.into();
    }
    let household = Household::new(snapshot).unwrap();
    let layout = House::of(&staging::lived_in(&household, "floor.boards", "wall.leafy").rooms);
    (household, layout)
}

fn run(life: &mut Life, household: &Household, layout: &House, from: f32, seconds: f32) -> f32 {
    run_liking(life, household, layout, &[], from, seconds)
}

fn run_liking(
    life: &mut Life,
    household: &Household,
    layout: &House,
    likings: &[Liking],
    from: f32,
    seconds: f32,
) -> f32 {
    let mut now = from;
    while now < from + seconds {
        now += 1.0 / 30.0;
        life.tick(
            household,
            layout,
            &household.snapshot,
            likings,
            now,
            1.0 / 30.0,
        );
    }
    now
}

fn piece(layout: &House, id: &str) -> u16 {
    layout
        .pieces
        .iter()
        .find(|p| p.piece.as_str() == id)
        .unwrap_or_else(|| panic!("no {id}"))
        .uid
}

#[test]
fn left_alone_everyone_finds_things_to_do_of_their_own_accord() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let mut seen = std::collections::BTreeSet::new();
    let mut now = 0.0;
    for _ in 0..240 {
        now = run(&mut life, &household, &layout, now, 1.0);
        for resident in &household.residents {
            if let Some((act, asked)) = life.current(resident.id) {
                assert!(!asked);
                seen.insert(format!("{:?}", std::mem::discriminant(&act)));
            }
        }
    }
    assert!(seen.len() >= 4, "only ever did {seen:?}");
}

#[test]
fn an_asked_act_is_done_and_then_the_resident_is_its_own_again() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let keeper = household.keeper().id;
    let chair = piece(&layout, "armchair");
    assert_eq!(life.ask(keeper, Act::Sit(chair), 0.0), Asked::Queued);
    let mut sat = false;
    let mut now = 0.0;
    for _ in 0..40 {
        now = run(&mut life, &household, &layout, now, 0.5);
        if life.actor(keeper).unwrap().on_piece == Some(chair) {
            sat = true;
            break;
        }
    }
    assert!(sat, "never sat in the armchair");
    assert_eq!(
        life.queue(keeper).len(),
        1,
        "the sitting is what the queue shows"
    );
    now = run(&mut life, &household, &layout, now, 15.0);
    assert!(life.queue(keeper).is_empty());
    assert!(
        life.take_events()
            .contains(&Event::Used(keeper, Used::Piece(chair))),
        "sitting in it counts as using it"
    );
    run(&mut life, &household, &layout, now, 10.0);
    assert!(life.current(keeper).is_none_or(|(_, asked)| !asked));
}

#[test]
fn the_queue_holds_three_and_says_so_when_it_is_full() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let keeper = household.keeper().id;
    for x in 0..3 {
        assert_eq!(life.ask(keeper, Act::GoTo(x + 2, 6), 0.0), Asked::Queued);
    }
    assert_eq!(life.ask(keeper, Act::GoTo(6, 6), 0.0), Asked::Full);
    life.cancel(keeper, 2, 0.0);
    assert_eq!(life.queue(keeper).len(), 2);
}

/// Ask the keeper for `act`, and whether it was ever seen doing `done`.
fn happens(act: Act, done: formiga_art::BodyClip) -> bool {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let mut life = Life::new(&household, &layout);
    life.ask(keeper, act, 0.0);
    let mut now = 0.0;
    for _ in 0..80 {
        now = run(&mut life, &household, &layout, now, 0.25);
        let actor = life.actor(keeper).unwrap();
        if !actor.walking() && actor.pose.clip == done {
            return true;
        }
    }
    false
}

#[test]
fn every_kind_of_asked_act_happens() {
    let (household, layout) = home();
    let pip = household.residents[1].id;
    let ball = piece(&layout, "ball");
    let table = piece(&layout, "low_table");
    let fern = piece(&layout, "fern");
    let rug = piece(&layout, "long_rug");
    let shelf = piece(&layout, "shelf");
    for (act, done) in [
        (Act::Play(ball), ActionKind::SoloPlay.into()),
        (
            Act::Inspect(DisplayId::find(0)),
            ActionKind::InspectScreen.into(),
        ),
        (Act::Greet(pip), ActionKind::Greet.into()),
        (Act::SitAt(table), ActionKind::Perch.into()),
        (Act::Tend(fern), Gesture::Reach.into()),
        (Act::Sprawl(rug), ActionKind::Sleep.into()),
        (Act::Browse(shelf), ActionKind::InspectScreen.into()),
        (Act::TryOn(DisplayId::find(1)), Gesture::Bop.into()),
    ] {
        assert!(happens(act.clone(), done), "{act:?} never happened");
    }
}

#[test]
fn a_find_tried_on_is_worn_for_a_while_and_put_back() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let key = DisplayId::find(1);
    let mut life = Life::new(&household, &layout);
    life.ask(keeper, Act::TryOn(key.clone()), 0.0);
    let mut now = 0.0;
    let mut worn = false;
    for _ in 0..80 {
        now = run(&mut life, &household, &layout, now, 0.25);
        if life.worn().contains(&key) {
            worn = true;
            break;
        }
    }
    assert!(worn, "the key was never put on");
    run(&mut life, &household, &layout, now, 15.0);
    assert!(life.worn().is_empty(), "the key was never put back");
}

#[test]
fn a_lamp_switched_twice_is_on_again() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let lamp = piece(&layout, "lamp");
    let mut life = Life::new(&household, &layout);
    life.ask(keeper, Act::SwitchLamp(lamp), 0.0);
    let mut now = 0.0;
    for _ in 0..60 {
        now = run(&mut life, &household, &layout, now, 0.25);
        if life.lamps_off().contains(&lamp) {
            break;
        }
    }
    assert!(life.lamps_off().contains(&lamp));
    life.ask(keeper, Act::SwitchLamp(lamp), now);
    for _ in 0..60 {
        now = run(&mut life, &household, &layout, now, 0.25);
        if !life.lamps_off().contains(&lamp) {
            break;
        }
    }
    assert!(!life.lamps_off().contains(&lamp));
}

#[test]
fn a_pat_lets_go_of_the_queue_and_autonomy_resumes() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let keeper = household.keeper().id;
    life.ask(keeper, Act::GoTo(6, 6), 0.0);
    life.ask(keeper, Act::GoTo(2, 6), 0.0);
    life.pet(&household, keeper, 0.1);
    assert!(life.queue(keeper).is_empty());
    run(&mut life, &household, &layout, 0.1, 12.0);
    assert!(life.current(keeper).is_some());
}

#[test]
fn a_resident_carried_and_put_down_lands_on_open_floor() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let pip = household.residents[1].id;
    life.pick_up(pip, 0.0);
    assert_eq!(life.held(), Some(pip));
    let bed = layout
        .pieces
        .iter()
        .find(|p| p.piece.as_str() == "bed")
        .unwrap();
    life.carry(pip, (f32::from(bed.x) + 0.5, f32::from(bed.y) + 0.5));
    life.put_down(&layout, pip, 0.5);
    let (x, y) = Floor::tile_of(life.actor(pip).unwrap().pos);
    assert!(Floor::of(&layout).open(x, y));
    assert_eq!(life.held(), None);
}

#[test]
fn arranging_holds_everyone_still_and_off_the_furniture() {
    let household = Household::new(sample::snapshot()).unwrap();
    let layout = House::of(&[starter::room(&household.snapshot)]);
    let mut life = Life::new(&household, &layout);
    let mut now = run(&mut life, &household, &layout, 0.0, 30.0);
    life.pause(&household, &layout, now);
    let before: Vec<_> = life.actors.iter().map(|a| a.pos).collect();
    now = run(&mut life, &household, &layout, now, 5.0);
    let after: Vec<_> = life.actors.iter().map(|a| a.pos).collect();
    assert_eq!(before, after);
    assert!(life.actors.iter().all(|a| a.on_piece.is_none()));
    life.resume(&layout, &household.snapshot, now);
    assert!(!life.paused());
}

#[test]
fn a_lazybones_rests_more_than_an_explorer() {
    let rested = |kind: TemperamentKind| {
        let (household, layout) = tempered(kind);
        let mut life = Life::new(&household, &layout);
        let mut now = 0.0;
        let mut resting = 0;
        for _ in 0..600 {
            now = run(&mut life, &household, &layout, now, 0.5);
            resting += household
                .residents
                .iter()
                .filter(|r| life.asleep(r.id))
                .count();
        }
        resting
    };
    let (lazy, explorer) = (
        rested(TemperamentKind::Lazybones),
        rested(TemperamentKind::Explorer),
    );
    assert!(lazy > explorer, "lazybones {lazy}, explorer {explorer}");
}

#[test]
fn every_target_offers_something_and_a_find_can_always_be_looked_at() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let snapshot = &household.snapshot;
    let present: Vec<Id> = household.residents.iter().map(|r| r.id).collect();
    for shown in &layout.shown {
        let acts = choices(
            &household,
            &layout,
            snapshot,
            &present,
            keeper,
            &crate::scene::Target::Shown(shown.item.clone()),
        );
        assert_eq!(acts.first(), Some(&Act::Inspect(shown.item.clone())));
    }
    for placed in &layout.pieces {
        let acts = choices(
            &household,
            &layout,
            snapshot,
            &present,
            keeper,
            &crate::scene::Target::Piece(placed.uid),
        );
        assert!(!acts.is_empty(), "{} offers nothing", placed.piece);
    }
    let pip = household.residents[1].id;
    let social = choices(
        &household,
        &layout,
        snapshot,
        &present,
        keeper,
        &crate::scene::Target::Resident(pip),
    );
    assert!(social.contains(&Act::Greet(pip)) && social.contains(&Act::Comfort(pip)));
}

#[test]
fn a_visitor_is_offered_a_seat_but_never_a_bed() {
    let (household, layout) = home();
    let visitor = household.visitors[0].id;
    let present: Vec<Id> = household.everyone().map(|r| r.id).collect();
    let bed = piece(&layout, "bed");
    let acts = choices(
        &household,
        &layout,
        &household.snapshot,
        &present,
        visitor,
        &crate::scene::Target::Piece(bed),
    );
    assert!(
        !acts
            .iter()
            .any(|act| matches!(act, Act::Sleep(_) | Act::CurlUp(_)))
    );
    let chair = piece(&layout, "armchair");
    let acts = choices(
        &household,
        &layout,
        &household.snapshot,
        &present,
        visitor,
        &crate::scene::Target::Piece(chair),
    );
    assert!(acts.contains(&Act::Sit(chair)));
}

#[test]
fn a_visitor_knocks_comes_in_looks_round_and_goes_home() {
    let (household, layout) = home();
    assert_eq!(household.visitors.len(), 2, "the sample lends two friends");
    let mut life = Life::new(&household, &layout);
    let visitor = household.visitors[0].id;
    assert!(life.actor(visitor).unwrap().hidden, "not here yet");
    assert!(!life.present().contains(&visitor));
    let mut now = 0.0;
    let mut arrived = false;
    let mut left = false;
    let mut looked = false;
    for _ in 0..(8 * 60 * 2) {
        now = run(&mut life, &household, &layout, now, 0.5);
        for event in life.take_events() {
            match event {
                Event::Arrived(id) if id == visitor => arrived = true,
                Event::Left(id) if id == visitor => left = true,
                Event::Used(id, _) => assert!(
                    !household.is_visitor(id),
                    "a visitor never comes to like things here"
                ),
                _ => {}
            }
        }
        if let Some((Act::Inspect(_) | Act::Browse(_), _)) = life.current(visitor) {
            looked = true;
        }
        if left {
            break;
        }
    }
    assert!(arrived, "the visitor never came");
    assert!(looked, "the visitor never looked at anything on show");
    assert!(left, "the visitor never went home");
    assert!(life.actor(visitor).unwrap().hidden);
    assert!(!life.present().contains(&visitor));
}

/// Run until the first visitor comes in; then what each resident does about it. `close` keeps the
/// visitors close friends, as Desktop lends them; otherwise they are only acquaintances.
fn welcomes(kind: TemperamentKind, close: bool) -> Vec<Option<Act>> {
    let mut snapshot = sample::snapshot();
    for resident in &mut snapshot.residents {
        resident.character.temperament = kind.into();
    }
    if !close {
        for pair in &mut snapshot.relationships {
            pair.affinity = formiga_travel::Band::Low;
        }
    }
    let household = Household::new(snapshot).unwrap();
    let layout = House::of(&staging::lived_in(&household, "floor.boards", "wall.leafy").rooms);
    let mut life = Life::new(&household, &layout);
    // Everyone wide awake when the knock comes: a sleeper is left to sleep.
    for mind in &mut life.minds {
        mind.drives[Drive::Rest.index()] = 0.0;
    }
    let mut now = 0.0;
    for _ in 0..240 {
        now = run(&mut life, &household, &layout, now, 0.25);
        if life
            .take_events()
            .iter()
            .any(|event| matches!(event, Event::Arrived(_)))
        {
            break;
        }
    }
    household
        .residents
        .iter()
        .map(|r| life.current(r.id).map(|(act, _)| act))
        .collect()
}

#[test]
fn a_sweetheart_goes_to_the_door_and_a_wallflower_finds_the_furthest_seat() {
    let sweet = welcomes(TemperamentKind::Sweetheart, false);
    assert!(
        sweet.iter().any(|act| matches!(act, Some(Act::Greet(_)))),
        "{sweet:?}"
    );
    let shy = welcomes(TemperamentKind::Wallflower, false);
    assert!(
        shy.iter()
            .any(|act| matches!(act, Some(Act::Sit(_) | Act::CurlUp(_)))),
        "{shy:?}"
    );
    // A wallflower still goes to the door for a close friend.
    let friends = welcomes(TemperamentKind::Wallflower, true);
    assert!(
        friends.iter().any(|act| matches!(act, Some(Act::Greet(_)))),
        "{friends:?}"
    );
}

#[test]
fn a_show_off_shows_its_finds_to_a_visitor() {
    let (household, layout) = tempered(TemperamentKind::Showoff);
    let mut life = Life::new(&household, &layout);
    let mut now = 0.0;
    let mut shown = false;
    for _ in 0..(5 * 60 * 2) {
        now = run(&mut life, &household, &layout, now, 0.5);
        let showing = household.residents.iter().any(|r| {
            matches!(life.current(r.id), Some((Act::ShowTo(_, to), _)) if household.is_visitor(to))
        });
        if showing {
            shown = true;
            break;
        }
    }
    assert!(shown, "nothing was ever shown to a visitor");
}

#[test]
fn a_favourite_is_chosen_more_often_than_it_would_be() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let chair = piece(&layout, "armchair");
    let likings = [Liking {
        resident: TravelerId(keeper),
        thing: Liked::Piece {
            room: 0,
            uid: chair,
        },
        uses: 6,
    }];
    assert_eq!(
        favourites(&likings, &layout, keeper),
        vec![(Kind::Seat, likings[0].thing.clone())]
    );
    let chosen = |likings: &[Liking]| {
        let mut life = Life::new(&household, &layout);
        life.minds[0].drives = [0.2, 0.2, 0.2, 0.2, 1.0];
        (0..400)
            .filter(|_| {
                life.minds[0].last = None;
                matches!(
                    life.choose(&household, &layout, &household.snapshot, likings, 0),
                    Some(Act::Sit(uid) | Act::Nap(uid)) if uid == chair
                )
            })
            .count()
    };
    let (loved, plain) = (chosen(&likings), chosen(&[]));
    assert!(loved > plain, "with a favourite {loved}, without {plain}");
}

#[test]
fn a_grump_grumbles_at_whoever_is_in_its_favourite_chair() {
    let (household, layout) = tempered(TemperamentKind::Grump);
    let keeper = household.keeper().id;
    let pip = household.residents[1].id;
    let chair = piece(&layout, "armchair");
    let likings = [Liking {
        resident: TravelerId(keeper),
        thing: Liked::Piece {
            room: 0,
            uid: chair,
        },
        uses: 6,
    }];
    let mut life = Life::new(&household, &layout);
    life.ask(pip, Act::Relax(chair), 0.0);
    let mut now = 0.0;
    for _ in 0..80 {
        now = run_liking(&mut life, &household, &layout, &likings, now, 0.25);
        if life.actor(pip).unwrap().on_piece == Some(chair) {
            break;
        }
    }
    assert_eq!(life.actor(pip).unwrap().on_piece, Some(chair));
    life.minds[0].drives = [0.2, 0.2, 0.2, 0.2, 1.0];
    let grumbled = (0..200).any(|_| {
        life.minds[0].last = None;
        life.choose(&household, &layout, &household.snapshot, &likings, 0)
            == Some(Act::GrumbleAt(pip))
    });
    assert!(grumbled);
}

/// The sample house, lived in, with a reading nook built behind it.
fn with_nook() -> (Household, House) {
    let household = Household::new(sample::snapshot()).unwrap();
    let home = staging::grown(
        staging::lived_in(&household, "floor.boards", "wall.leafy"),
        2,
        &household.snapshot,
    );
    (household, House::of(&home.rooms))
}

#[test]
fn someone_asked_to_sit_in_another_room_walks_through_the_doorway_to_get_there() {
    let (household, house) = with_nook();
    let keeper = household.keeper().id;
    let chair = house
        .pieces
        .iter()
        .find(|placed| {
            placed.piece.as_str() == "armchair" && house.local(placed.uid).unwrap().0 == 1
        })
        .expect("the nook has its armchair")
        .uid;
    let doorway = house
        .walls
        .iter()
        .find(|wall| wall.door && wall.beyond.is_some())
        .copied()
        .expect("a doorway into the nook");
    let mut life = Life::new(&household, &house);
    assert_eq!(life.ask(keeper, Act::Sit(chair), 0.0), Asked::Queued);
    let mut now = 0.0;
    let mut through = false;
    for _ in 0..120 {
        now = run(&mut life, &household, &house, now, 0.25);
        let actor = life.actor(keeper).unwrap();
        let tile = Floor::tile_of(actor.pos);
        through |= tile == (i32::from(doorway.x), i32::from(doorway.y)) || tile == doorway.behind();
        // Never anywhere but on the house's floor.
        assert!(house.room_of_point(actor.pos).is_some(), "{:?}", actor.pos);
        if actor.on_piece == Some(chair) {
            break;
        }
    }
    assert_eq!(life.actor(keeper).unwrap().on_piece, Some(chair));
    assert!(through, "never went through the doorway");
}

#[test]
fn a_visitor_comes_in_at_the_front_door() {
    let (household, house) = with_nook();
    let door = *house.front_door().expect("a front door");
    let visitor = household.visitors[0].id;
    let mut life = Life::new(&household, &house);
    let mut now = 0.0;
    for _ in 0..400 {
        now = run(&mut life, &household, &house, now, 0.1);
        if life.present().contains(&visitor) {
            break;
        }
    }
    let at = life.actor(visitor).unwrap().pos;
    let (mx, my) = door.middle();
    assert!(
        (at.0 - mx).abs() < 1.2 && (at.1 - my).abs() < 1.2,
        "came in at {at:?}, the door is at {:?}",
        (mx, my)
    );
}

#[test]
fn in_a_house_of_rooms_everyone_gets_about_it() {
    let household = Household::new(sample::snapshot()).unwrap();
    let home = staging::grown(
        staging::lived_in(&household, "floor.boards", "wall.leafy"),
        3,
        &household.snapshot,
    );
    let house = House::of(&home.rooms);
    let mut life = Life::new(&household, &house);
    let mut visited = std::collections::BTreeSet::new();
    let mut now = 0.0;
    for _ in 0..(10 * 60) {
        now = run(&mut life, &household, &house, now, 1.0);
        for someone in household.everyone() {
            let actor = life.actor(someone.id).unwrap();
            if let Some(room) = house.room_of_point(actor.pos).filter(|_| !actor.hidden) {
                visited.insert(room);
            }
        }
    }
    assert_eq!(visited.len(), 3, "only ever in {visited:?}");
}
