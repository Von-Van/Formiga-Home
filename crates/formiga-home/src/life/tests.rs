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
            &[],
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
            &[],
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
        &[],
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
        &[],
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
        &[],
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
                Event::Left(id, _) if id == visitor => left = true,
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

#[test]
fn residents_spend_a_while_in_every_room_not_only_the_one_with_most_in_it() {
    let household = Household::new(sample::snapshot()).unwrap();
    let home = staging::grown(
        staging::lived_in(&household, "floor.boards", "wall.leafy"),
        3,
        &household.snapshot,
    );
    let house = House::of(&home.rooms);
    let mut life = Life::new(&household, &house);
    let mut seconds = std::collections::BTreeMap::new();
    let mut now = 0.0;
    for _ in 0..(20 * 60) {
        now = run(&mut life, &household, &house, now, 1.0);
        for resident in &household.residents {
            let actor = life.actor(resident.id).unwrap();
            if let Some(room) = house.room_of_point(actor.pos).filter(|_| !actor.hidden) {
                *seconds.entry((resident.id, room)).or_insert(0) += 1;
            }
        }
    }
    for resident in &household.residents {
        for room in 0..house.rooms.len() as u8 {
            let there = seconds.get(&(resident.id, room)).copied().unwrap_or(0);
            assert!(
                there >= 2 * 60,
                "{} spent {there} s of 20 minutes in room {room}",
                resident.name
            );
        }
    }
}

#[test]
fn a_little_one_asked_to_draw_draws_its_own_adult_and_says_so_when_done() {
    let (household, layout) = home();
    let (keeper, pip) = (household.keeper().id, household.residents[1].id);
    let table = piece(&layout, "low_table");
    let present = [keeper, pip];
    let offered = choices(
        &household,
        &layout,
        &household.snapshot,
        &present,
        &[],
        pip,
        &crate::scene::Target::Piece(table),
    );
    assert!(offered.contains(&Act::Draw(table, keeper)), "{offered:?}");
    let grown_up = choices(
        &household,
        &layout,
        &household.snapshot,
        &present,
        &[],
        keeper,
        &crate::scene::Target::Piece(table),
    );
    assert!(
        !grown_up.iter().any(|act| matches!(act, Act::Draw(..))),
        "drawing is for little ones"
    );
    let mut life = Life::new(&household, &layout);
    life.ask(pip, Act::Draw(table, keeper), 0.0);
    let mut now = 0.0;
    let mut drew = false;
    for _ in 0..(60 * 2) {
        now = run(&mut life, &household, &layout, now, 0.5);
        if life.take_events().contains(&Event::Drew(pip, keeper)) {
            drew = true;
            break;
        }
    }
    assert!(drew, "the drawing was never finished");
}

#[test]
fn left_to_itself_a_little_one_draws_once_a_visit_at_most() {
    let (household, layout) = home();
    let pip = household.residents[1].id;
    let mut life = Life::new(&household, &layout);
    let mut drawings = 0;
    let mut now = 0.0;
    for _ in 0..(30 * 60) {
        now = run(&mut life, &household, &layout, now, 1.0);
        drawings += life
            .take_events()
            .iter()
            .filter(|event| matches!(event, Event::Drew(by, _) if *by == pip))
            .count();
    }
    assert!(drawings <= 1, "drew {drawings} of its own accord");
}

#[test]
fn friends_who_drop_by_spend_time_with_the_household_and_it_is_counted_within_bounds() {
    let (household, layout) = home();
    let mut life = Life::new(&household, &layout);
    let mut now = 0.0;
    for _ in 0..(12 * 60) {
        now = run(&mut life, &household, &layout, now, 1.0);
    }
    let together = life.together();
    assert!(!together.is_empty(), "nobody spent any time together");
    for (a, b, _, times) in together {
        assert!(a < b, "each pair once, the lesser first");
        assert!((1..=formiga_home_contract::limits::MAX_TOGETHER).contains(&times));
    }
}

#[test]
fn what_a_friend_leaves_suits_it_and_a_warm_friend_leaves_something_more_often() {
    use crate::keepsakes::{gift, gives};
    let mut character = home().0.visitors[0].character.clone();
    character.kind = TemperamentKind::Explorer;
    assert_eq!(gift(&character), MementoKind::Postcard);
    character.kind = TemperamentKind::Showoff;
    assert_eq!(gift(&character), MementoKind::Rosette);
    character.kind = TemperamentKind::Sweetheart;
    assert_eq!(gift(&character), MementoKind::JamJar);
    assert!(gives(&character, 1.0) > gives(&character, 0.0));
    let sweet = gives(&character, 0.66);
    character.kind = TemperamentKind::Grump;
    assert!(gives(&character, 0.66) < sweet);
}

/// The lived-in house with a guest bedroll put down wherever it first fits.
fn with_bedroll() -> (Household, House, u16) {
    let household = Household::new(sample::snapshot()).unwrap();
    let mut home = staging::lived_in(&household, "floor.boards", "wall.leafy");
    let bedroll = catalog::PIECES.iter().find(|p| p.id == "bedroll").unwrap();
    let (x, y) = (0..8)
        .flat_map(|y| (0..8).map(move |x| (x, y)))
        .find(|&(x, y)| room::can_place(&home.rooms[0], bedroll, x, y, 0, None))
        .expect("somewhere for a bedroll");
    let uid = room::add_piece(&mut home, 0, bedroll, x, y, 0);
    (household, House::of(&home.rooms), uid)
}

#[test]
fn a_friend_asked_to_stay_over_stays_the_visit_and_sleeps_in_the_guest_bedroll() {
    let (household, layout, bedroll) = with_bedroll();
    let friend = household.visitors[0].id;
    let mut life = Life::new(&household, &layout);
    let mut now = 0.0;
    while !life.present().contains(&friend) {
        now = run(&mut life, &household, &layout, now, 1.0);
        assert!(now < 120.0, "the friend never came");
    }
    assert!(!life.staying(friend));
    assert!(
        life.ask_to_stay(&household, friend),
        "a close friend agrees"
    );
    assert!(life.take_events().contains(&Event::StayingOver(friend)));
    assert_eq!(life.guests(), vec![friend]);
    let mut slept = false;
    for _ in 0..(10 * 60) {
        now = run(&mut life, &household, &layout, now, 1.0);
        assert!(life.present().contains(&friend), "went home after all");
        if let Some((Act::Sleep(uid) | Act::CurlUp(uid), _)) = life.current(friend) {
            assert_eq!(uid, bedroll, "a guest sleeps only where it may");
            slept = true;
        }
    }
    assert!(slept, "the friend never turned in");
    let offered = choices(
        &household,
        &layout,
        &household.snapshot,
        &life.present(),
        &life.guests(),
        friend,
        &crate::scene::Target::Piece(bedroll),
    );
    assert!(offered.contains(&Act::Sleep(bedroll)), "{offered:?}");
}

#[test]
fn only_a_close_friend_talks_a_shy_one_into_staying_over() {
    let mut character = home().0.visitors[0].character.clone();
    character.kind = TemperamentKind::Wallflower;
    assert!(!character.agrees_to_stay(0.66));
    assert!(character.agrees_to_stay(1.0));
    character.kind = TemperamentKind::Sweetheart;
    assert!(character.agrees_to_stay(0.66));
    assert!(character.asks_to_stay(1.0) > character.asks_to_stay(0.0));
}

#[test]
fn at_night_the_household_turns_in_and_sleeps_longer_than_by_day() {
    let (household, layout) = home();
    let asleep_for = |dark: f32| {
        let mut life = Life::new(&household, &layout);
        life.set_dark(dark);
        let mut now = 0.0;
        let mut asleep = 0;
        for _ in 0..(10 * 60) {
            now = run(&mut life, &household, &layout, now, 1.0);
            asleep += household
                .residents
                .iter()
                .filter(|resident| life.asleep(resident.id))
                .count();
        }
        asleep
    };
    let (day, night) = (asleep_for(0.0), asleep_for(0.55));
    assert!(
        night > day * 3 / 2 + 30,
        "asleep {night} s by night, {day} s by day"
    );
}

/// Every tick of `seconds` of life, with a pat, a lift or an errand now and then: whether anyone
/// in the house moved without walking, and whether anyone walked without moving.
fn glides_or_marks_time(household: &Household, layout: &House, seconds: f32) -> Vec<String> {
    let mut life = Life::new(household, layout);
    let mut faults = Vec::new();
    let mut in_place = vec![0; life.actors.len()];
    let mut dice = 12345u64;
    let mut carried: Option<(Id, f32)> = None;
    let dt = 1.0 / 30.0;
    let mut now = 0.0;
    for tick in 0..(seconds / dt) as u32 {
        if tick % 97 == 0 && carried.is_none() {
            dice = dice
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let present = life.present();
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
        if let Some((who, since)) = carried
            && now - since > 1.0
        {
            life.put_down(layout, who, now);
            carried = None;
        }
        let before: Vec<_> = life
            .actors
            .iter()
            .map(|a| (a.pos, a.hidden, a.walking()))
            .collect();
        now += dt;
        life.tick(household, layout, &household.snapshot, &[], now, dt);
        for (index, actor) in life.actors.iter().enumerate() {
            let (was, hidden, was_walking) = before[index];
            if hidden || actor.hidden || carried.is_some_and(|(who, _)| who == actor.id) {
                continue;
            }
            let moved = ((actor.pos.0 - was.0).powi(2) + (actor.pos.1 - was.1).powi(2)).sqrt();
            let walks = actor.shows(now) == formiga_art::BodyClip::Action(ActionKind::Traverse);
            if moved > 0.2 {
                faults.push(format!("{} jumped {moved:.1} tiles at {now:.1}s", actor.id));
            } else if moved > 1e-4 && !walks && !was_walking {
                faults.push(format!("{} glided at {now:.1}s", actor.id));
            }
            // The tick a walk ends in may finish it with a step; the tick one begins in is spent
            // setting off. Any longer on the spot is marking time.
            in_place[index] = if walks && moved <= 1e-4 {
                in_place[index] + 1
            } else {
                0
            };
            if in_place[index] == 3 {
                faults.push(format!("{} walked on the spot at {now:.1}s", actor.id));
            }
        }
    }
    faults
}

#[test]
fn nobody_glides_across_the_floor_or_is_lifted_onto_a_seat_from_across_the_room() {
    let (household, layout) = home();
    let faults = glides_or_marks_time(&household, &layout, 20.0 * 60.0);
    assert!(faults.is_empty(), "{faults:#?}");
    let (household, layout) = with_nook();
    let faults = glides_or_marks_time(&household, &layout, 20.0 * 60.0);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn a_seat_is_climbed_onto_at_the_end_of_the_walk_and_walked_off_when_done() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let mut life = Life::new(&household, &layout);
    let sofa = piece(&layout, "sofa");
    life.ask(keeper, Act::Sit(sofa), 0.0);
    let mut now = 0.0;
    let mut climbing = false;
    let mut sat = false;
    while now < 30.0 && !sat {
        now = run(&mut life, &household, &layout, now, 1.0 / 30.0);
        let actor = life.actor(keeper).unwrap();
        if actor.walking() && actor.on_piece == Some(sofa) && actor.lift > 0.0 {
            climbing = true;
        }
        sat = !actor.walking() && actor.pose.clip == ActionKind::Perch.into();
    }
    assert!(climbing, "it was never seen stepping up onto the sofa");
    assert!(sat, "it never sat");
    let seat = life.actor(keeper).unwrap().pos;
    // Left to finish sitting, and then to get on with things.
    let mut stepped_down = false;
    for _ in 0..(120 * 30) {
        now = run(&mut life, &household, &layout, now, 1.0 / 30.0);
        let actor = life.actor(keeper).unwrap();
        let away = ((actor.pos.0 - seat.0).powi(2) + (actor.pos.1 - seat.1).powi(2)).sqrt();
        if actor.walking() && actor.on_piece == Some(sofa) && away > 0.05 {
            stepped_down = true;
        }
        if actor.on_piece.is_none() {
            break;
        }
    }
    assert!(stepped_down, "it never walked down off the sofa");
}

#[test]
fn a_pat_stops_a_walk_so_the_pat_is_answered() {
    let (household, layout) = home();
    let keeper = household.keeper().id;
    let mut life = Life::new(&household, &layout);
    life.ask(keeper, Act::GoTo(7, 7), 0.0);
    let now = run(&mut life, &household, &layout, 0.0, 0.3);
    assert!(life.actor(keeper).unwrap().walking());
    life.pet(&household, keeper, now);
    let actor = life.actor(keeper).unwrap();
    assert!(!actor.walking());
    assert_eq!(actor.shows(now), ActionKind::PetReaction.into());
}
