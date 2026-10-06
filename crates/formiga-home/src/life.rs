//! The household's life in the room: what each resident does, whether the owner asked or it
//! decided for itself, and the friends who drop by.
//!
//! The owner can ask anyone in the house for up to three things in turn. Whenever it has nothing
//! asked of it, it chooses for itself, from what the room offers and what it feels like: its
//! drives, its temperament, its habits, what it has come to like, and how it gets on with whoever
//! else is there. A lazybones finds the bed or sprawls on the rug, an explorer the newest and
//! oddest thing on the shelf, a show-off someone to show it to; close friends seek each other out
//! and a little one keeps near its adult. Nothing here is a chore and nothing goes wrong if the
//! owner does nothing at all.
//!
//! Visitors are friends Desktop has lent for the visit. They knock a little while after the house
//! opens, come in, have a good look at whatever is on show, spend time with whoever they are
//! closest to, and after a few minutes go home again. The household answers in character: a
//! sweetheart is first to the door, a wallflower finds the furthest seat, a show-off has something
//! to show them.

use crate::actor::{Actor, Pose};
use crate::art::cues::Cue;
use crate::catalog::{self, Family, Use};
use crate::character::Drive;
use crate::house::{At, House};
use crate::household::{Household, Id};
use crate::keepsakes;
use crate::path::Floor;
use crate::room;
use formiga_art::{AccessoryArt, ExpressionKind};
use formiga_core::{Accessory, ActionKind, Gesture, Habit, TemperamentKind};
use formiga_home_contract::limits::MAX_TOGETHER;
use formiga_home_contract::{
    DisplayId, DisplayItem, DisplayMode, DisplaySource, HomeSnapshot, Liked, Liking, MementoKind,
    Together, TravelerId,
};
use formiga_travel::{Band, Trait};
use std::collections::{BTreeMap, VecDeque};

/// How many things the owner can ask of one resident at once, the one under way included.
pub const QUEUE_LIMIT: usize = 3;

/// How long in one room before another calls, in seconds: for a homebody, and for one who gets
/// about the house most.
const ROOM_RESTLESS: (f32, f32) = (220.0, 70.0);

/// How likely, once a room has palled, the next thing it thinks of doing is somewhere else.
const LEAVE_ROOM: f32 = 0.65;

/// How many times something must be chosen before it is a favourite.
pub const FAVOURITE_AFTER: u16 = 3;

/// Something someone in the house does.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    GoTo(u8, u8),
    Sit(u16),
    Relax(u16),
    Nap(u16),
    Sleep(u16),
    CurlUp(u16),
    /// Lie down in the bed, and invite a little one to curl up too.
    InviteLittle(u16, Id),
    /// Sit on the floor at a table.
    SitAt(u16),
    /// A little one's: sit at a table and draw someone.
    Draw(u16, Id),
    /// Bounce on a bed, which is not what beds are for.
    Bounce(u16),
    /// Stretch out on a rug.
    Sprawl(u16),
    /// Switch a lamp on, or off.
    SwitchLamp(u16),
    /// Tend a plant.
    Tend(u16),
    /// Look over everything on a shelf or in a case.
    Browse(u16),
    Play(u16),
    PlayWith(u16, Id),
    ShowOffToy(u16),
    Snack(u16),
    Share(u16, Id),
    Inspect(DisplayId),
    Remember(DisplayId),
    ShowTo(DisplayId, Id),
    FussWith(DisplayId),
    PlayWithFind(DisplayId),
    /// Wear a find for a while, then put it back.
    TryOn(DisplayId),
    Greet(Id),
    SitTogether(Id),
    PlayTogether(Id),
    Tease(Id),
    Comfort(Id),
    Hug(Id),
    /// Its own idea only: a grump put out, for instance, by someone in its favourite seat.
    GrumbleAt(Id),
    /// Its own idea only: a look round somewhere else in the room.
    Wander(u8, u8),
}

impl Act {
    /// Who else it takes, if anyone.
    pub fn partner(&self) -> Option<Id> {
        match self {
            Self::InviteLittle(_, other)
            | Self::PlayWith(_, other)
            | Self::Share(_, other)
            | Self::ShowTo(_, other)
            | Self::Greet(other)
            | Self::SitTogether(other)
            | Self::PlayTogether(other)
            | Self::Tease(other)
            | Self::Comfort(other)
            | Self::Hug(other) => Some(*other),
            _ => None,
        }
    }

    pub fn piece(&self) -> Option<u16> {
        match self {
            Self::Sit(uid)
            | Self::Relax(uid)
            | Self::Nap(uid)
            | Self::Sleep(uid)
            | Self::CurlUp(uid)
            | Self::InviteLittle(uid, _)
            | Self::SitAt(uid)
            | Self::Draw(uid, _)
            | Self::Bounce(uid)
            | Self::Sprawl(uid)
            | Self::SwitchLamp(uid)
            | Self::Tend(uid)
            | Self::Browse(uid)
            | Self::Play(uid)
            | Self::PlayWith(uid, _)
            | Self::ShowOffToy(uid)
            | Self::Snack(uid)
            | Self::Share(uid, _) => Some(*uid),
            _ => None,
        }
    }

    pub fn item(&self) -> Option<&DisplayId> {
        match self {
            Self::Inspect(item)
            | Self::Remember(item)
            | Self::ShowTo(item, _)
            | Self::FussWith(item)
            | Self::PlayWithFind(item)
            | Self::TryOn(item) => Some(item),
            _ => None,
        }
    }

    /// How it is time spent together, as Desktop counts it, if it is.
    fn together(&self) -> Option<Together> {
        match self {
            Self::PlayTogether(_) | Self::PlayWith(..) => Some(Together::Play),
            Self::SitTogether(_) | Self::Share(..) | Self::Hug(_) | Self::InviteLittle(..) => {
                Some(Together::Cozy)
            }
            Self::Comfort(_) => Some(Together::Care),
            Self::Tease(_) => Some(Together::Squabble),
            _ => None,
        }
    }

    /// Done sitting or lying on its piece.
    fn settles(&self) -> bool {
        matches!(
            self,
            Self::Sit(_)
                | Self::Relax(_)
                | Self::Nap(_)
                | Self::Sleep(_)
                | Self::CurlUp(_)
                | Self::InviteLittle(..)
                | Self::Bounce(_)
        )
    }

    /// The drive doing it settles.
    fn drive(&self) -> Drive {
        match self {
            Self::Sit(_)
            | Self::Relax(_)
            | Self::CurlUp(_)
            | Self::Snack(_)
            | Self::Remember(_)
            | Self::SitAt(_)
            | Self::Tend(_)
            | Self::GrumbleAt(_) => Drive::Comfort,
            Self::Nap(_) | Self::Sleep(_) | Self::InviteLittle(..) | Self::Sprawl(_) => Drive::Rest,
            Self::Play(_)
            | Self::PlayWith(..)
            | Self::ShowOffToy(_)
            | Self::PlayWithFind(_)
            | Self::PlayTogether(_)
            | Self::Tease(_)
            | Self::FussWith(_)
            | Self::TryOn(_)
            | Self::Draw(..)
            | Self::Bounce(_) => Drive::Play,
            Self::Inspect(_)
            | Self::Browse(_)
            | Self::SwitchLamp(_)
            | Self::Wander(..)
            | Self::GoTo(..) => Drive::Curiosity,
            Self::Share(..)
            | Self::ShowTo(..)
            | Self::Greet(_)
            | Self::SitTogether(_)
            | Self::Comfort(_)
            | Self::Hug(_) => Drive::Company,
        }
    }

    /// What the owner is offered, and what the queue shows: "Nap in the armchair", "Hug Pip".
    pub fn label(&self, household: &Household, house: &House, snapshot: &HomeSnapshot) -> String {
        let piece = |uid: &u16| {
            house
                .piece(*uid)
                .and_then(|placed| catalog::piece(&placed.piece))
                .map_or("something", |piece| piece.name)
                .to_lowercase()
        };
        let item = |id: &DisplayId| {
            snapshot
                .item(id)
                .map_or("something".to_owned(), |item| item.name.to_lowercase())
        };
        let who = |id: &Id| {
            household
                .resident(*id)
                .map_or("someone".to_owned(), |r| r.name.clone())
        };
        match self {
            Self::GoTo(..) => "Go here".to_owned(),
            Self::Sit(uid) => format!("Sit on the {}", piece(uid)),
            Self::Relax(uid) => format!("Relax on the {}", piece(uid)),
            Self::Nap(uid) => format!("Nap on the {}", piece(uid)),
            Self::Sleep(uid) => format!("Sleep in the {}", piece(uid)),
            Self::CurlUp(uid) => format!("Curl up in the {}", piece(uid)),
            Self::InviteLittle(_, little) => format!("Turn in with {}", who(little)),
            Self::SitAt(uid) => format!("Sit at the {}", piece(uid)),
            Self::Draw(uid, of) => format!("Draw {} at the {}", who(of), piece(uid)),
            Self::Bounce(uid) => format!("Bounce on the {}", piece(uid)),
            Self::Sprawl(uid) => format!("Stretch out on the {}", piece(uid)),
            Self::SwitchLamp(uid) => format!("Switch the {}", piece(uid)),
            Self::Tend(uid) => format!("Tend the {}", piece(uid)),
            Self::Browse(uid) => format!("Look over the {}", piece(uid)),
            Self::Play(uid) => format!("Play with the {}", piece(uid)),
            Self::PlayWith(uid, other) => {
                format!("Play with the {} with {}", piece(uid), who(other))
            }
            Self::ShowOffToy(uid) => format!("Show off with the {}", piece(uid)),
            Self::Snack(uid) => format!("Have a snack from the {}", piece(uid)),
            Self::Share(_, other) => format!("Share a snack with {}", who(other)),
            Self::Inspect(id) => format!("Look at the {}", item(id)),
            Self::Remember(id) => format!("Remember the {}", item(id)),
            Self::ShowTo(id, other) => format!("Show {} the {}", who(other), item(id)),
            Self::FussWith(id) => format!("Fuss with the {}", item(id)),
            Self::PlayWithFind(id) => format!("Play with the {}", item(id)),
            Self::TryOn(id) => format!("Try on the {}", item(id)),
            Self::Greet(other) => format!("Say hello to {}", who(other)),
            Self::SitTogether(other) => format!("Sit with {}", who(other)),
            Self::PlayTogether(other) => format!("Play with {}", who(other)),
            Self::Tease(other) => format!("Tease {}", who(other)),
            Self::Comfort(other) => format!("Comfort {}", who(other)),
            Self::Hug(other) => format!("Hug {}", who(other)),
            Self::GrumbleAt(other) => format!("Grumble at {}", who(other)),
            Self::Wander(..) => "Have a look round".to_owned(),
        }
    }

    /// What it is doing, as the room's status line says it: "napping on the armchair".
    fn doing(&self, household: &Household, house: &House, snapshot: &HomeSnapshot) -> String {
        let label = self.label(household, house, snapshot);
        let mut words = label.splitn(2, ' ');
        let verb = words.next().unwrap_or_default();
        let rest = words.next().unwrap_or_default();
        let ing = match verb {
            "Go" => return "on the way somewhere".to_owned(),
            "Sit" => "sitting",
            "Relax" => "relaxing",
            "Nap" => "napping",
            "Sleep" => "sleeping",
            "Curl" => "curling",
            "Turn" => "turning",
            "Bounce" => "bouncing",
            "Draw" => "drawing",
            "Stretch" => "stretching",
            "Switch" => "switching",
            "Tend" => "tending",
            "Play" => "playing",
            "Show" => "showing",
            "Have" => "having",
            "Share" => "sharing",
            "Look" => "looking",
            "Remember" => "remembering",
            "Fuss" => "fussing",
            "Try" => "trying",
            "Say" => "saying",
            "Tease" => "teasing",
            "Comfort" => "comforting",
            "Hug" => "hugging",
            "Grumble" => "grumbling",
            other => return other.to_lowercase(),
        };
        format!("{ing} {rest}")
    }
}

/// What happened that the window wants to know about.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A visitor has come in.
    Arrived(Id),
    /// A visitor has gone home, leaving a keepsake of this kind, if it left one.
    Left(Id, Option<MementoKind>),
    /// A little one has finished a drawing of someone.
    Drew(Id, Id),
    /// A visitor is staying over.
    StayingOver(Id),
    /// A resident has used something in its home once more: a piece, by its name in the house,
    /// or something shown.
    Used(Id, Used),
}

/// What was used.
#[derive(Clone, Debug, PartialEq)]
pub enum Used {
    Piece(u16),
    Shown(DisplayId),
}

/// How a resident takes part in someone else's act.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Leads,
    Joins(Id),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Walking to where it is done.
    Going,
    /// There, waiting for whoever it is done with.
    Waiting {
        since: f32,
    },
    /// A habit's little flourish before settling: a stretch, a turn about.
    Prelude {
        until: f32,
    },
    Doing {
        until: f32,
    },
}

/// A place to sit or lie on a piece: which piece, where on it, how high, and which way someone on
/// it faces across the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seat {
    piece: u16,
    at: (f32, f32),
    lift: f32,
    facing_right: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Plan {
    act: Act,
    part: Part,
    asked: bool,
    stage: Stage,
    /// Where it is done from, once it is there: a seat on a piece, or standing beside.
    seat: Option<Seat>,
    /// What it looks at while it does it.
    face: Option<(f32, f32)>,
}

/// Whether someone is in the house.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Presence {
    /// Lives here.
    Home,
    /// A visitor who will knock at `at`.
    Expected { at: f32 },
    /// A visitor in the house until `until`.
    Visiting { until: f32 },
    /// A visitor on the way out.
    Leaving,
    /// A visitor gone home.
    Gone,
}

impl Presence {
    fn in_house(self) -> bool {
        matches!(self, Self::Home | Self::Visiting { .. })
    }
}

/// One person's inner life.
struct Mind {
    id: Id,
    presence: Presence,
    drives: [f32; 5],
    queue: VecDeque<Act>,
    plan: Option<Plan>,
    /// Free to choose again from this moment.
    dawdle_until: f32,
    /// What it chose for itself last, so it does not do the same thing twice running.
    last: Option<Act>,
    dice: Dice,
    /// Held by the owner, or just put down: nothing else happens until it is over.
    handled: Option<Handled>,
    /// What it has had a good look at since it came in.
    seen: Vec<DisplayId>,
    /// The room it is in, and since when; and how restless that has made it, from nothing just
    /// in to everything after a few minutes, for the other rooms to come to mind.
    room: Option<u8>,
    entered: f32,
    restless: f32,
    /// Has drawn something of its own accord since it came in: once is a keepsake, more would
    /// fill the house.
    drew: bool,
    /// A visitor staying over: it stays as long as the house is open, and sleeps here.
    staying: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Handled {
    Petted { until: f32 },
    Held,
    Landing { until: f32 },
}

/// A small, steady source of whims, one per person, so a household's life plays out the same for
/// the same start.
#[derive(Clone, Debug)]
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 40) as f32 / (1_u64 << 24) as f32
    }

    pub fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
    }
}

/// What the owner asking for something came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Queued,
    Full,
}

/// What a favourite is a favourite of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Seat,
    Bed,
    Toy,
    Find,
}

/// What kind of thing in a home `thing` is, if it is one that can be a favourite.
pub fn kind_of(house: &House, thing: &Liked) -> Option<Kind> {
    match thing {
        Liked::Shown { .. } => Some(Kind::Find),
        Liked::Piece { .. } => {
            let piece = catalog::piece(&house.piece(house.of_liked(thing)?)?.piece)?;
            if piece.has(Use::Sleep) {
                Some(Kind::Bed)
            } else if piece.has(Use::Sit { seats: 1 }) {
                Some(Kind::Seat)
            } else if piece.has(Use::Play) {
                Some(Kind::Toy)
            } else {
                None
            }
        }
    }
}

/// A resident's favourites: of each kind, what it has chosen most, once it has chosen it often
/// enough.
pub fn favourites(likings: &[Liking], house: &House, resident: Id) -> Vec<(Kind, Liked)> {
    let mut best: Vec<(Kind, Liked, u16)> = Vec::new();
    for liking in likings {
        if liking.resident != TravelerId(resident) || liking.uses < FAVOURITE_AFTER {
            continue;
        }
        let Some(kind) = kind_of(house, &liking.thing) else {
            continue;
        };
        match best.iter_mut().find(|(known, ..)| *known == kind) {
            Some(entry) if liking.uses > entry.2 => {
                *entry = (kind, liking.thing.clone(), liking.uses)
            }
            Some(_) => {}
            None => best.push((kind, liking.thing.clone(), liking.uses)),
        }
    }
    best.into_iter()
        .map(|(kind, thing, _)| (kind, thing))
        .collect()
}

/// What `liked` is called, for the drawer: "the armchair", "the shell".
pub fn name_of(house: &House, snapshot: &HomeSnapshot, liked: &Liked) -> String {
    match liked {
        Liked::Piece { .. } => house
            .of_liked(liked)
            .and_then(|name| house.piece(name))
            .and_then(|placed| catalog::piece(&placed.piece))
            .map_or("something".to_owned(), |piece| {
                format!("the {}", piece.name.to_lowercase())
            }),
        Liked::Shown { item } => snapshot.item(item).map_or("something".to_owned(), |item| {
            format!("the {}", item.name.to_lowercase())
        }),
    }
}

pub struct Life {
    pub actors: Vec<Actor>,
    minds: Vec<Mind>,
    paused: bool,
    reduce_motion: bool,
    events: Vec<Event>,
    /// Lamps someone has switched off.
    lamps_off: Vec<u16>,
    /// Finds being worn for a while, and by whom.
    worn: Vec<(DisplayId, Id)>,
    /// What was on show the last time the room was arranged, so whatever is new since stands
    /// out.
    known: Vec<DisplayId>,
    novel: Vec<DisplayId>,
    /// How often each pair has spent time together, and how, since the house opened: two
    /// residents in turn, the lesser id first, each count no more than the contract keeps.
    together: BTreeMap<(Id, Id, Together), u8>,
}

impl Life {
    /// The household come home: everyone somewhere about the first room, already minded to do
    /// something, and any visitors due a little later.
    pub fn new(household: &Household, house: &House) -> Self {
        let floor = Floor::of(house);
        let reduce_motion = household.reduce_motion();
        let mut taken: Vec<(i32, i32)> = Vec::new();
        let mut actors = Vec::new();
        let mut minds = Vec::new();
        let first = &house.rooms[0];
        for (index, resident) in household.residents.iter().enumerate() {
            let mut dice = Dice::new(resident.id);
            let wish = (
                i32::from(first.x) + (2.0 + dice.between(0.0, f32::from(first.width) - 3.0)) as i32,
                i32::from(first.y) + (2.0 + dice.between(0.0, f32::from(first.depth) - 3.0)) as i32,
            );
            let mut tile = floor.nearest_open_in(wish, 0).unwrap_or(wish);
            if taken.contains(&tile) {
                tile = floor
                    .beside(tile.0 as u8, tile.1 as u8, 1, 1, (0, 1))
                    .into_iter()
                    .find(|t| !taken.contains(t))
                    .unwrap_or(tile);
            }
            taken.push(tile);
            let pos = (tile.0 as f32 + 0.5, tile.1 as f32 + 0.5);
            actors.push(Actor::new(resident, pos, reduce_motion));
            minds.push(Mind::new(
                resident.id,
                Presence::Home,
                0.4 + index as f32 * 0.7,
                dice,
            ));
        }
        let (outside, _) = door(house, &floor);
        for (index, visitor) in household.visitors.iter().enumerate() {
            let mut dice = Dice::new(visitor.id);
            let at = 22.0 + index as f32 * 14.0 + dice.between(0.0, 10.0);
            let mut actor = Actor::new(visitor, outside, reduce_motion);
            actor.hidden = true;
            actors.push(actor);
            minds.push(Mind::new(visitor.id, Presence::Expected { at }, at, dice));
        }
        let known = house.shown.iter().map(|shown| shown.item.clone()).collect();
        Self {
            actors,
            minds,
            paused: false,
            reduce_motion,
            events: Vec::new(),
            lamps_off: Vec::new(),
            worn: Vec::new(),
            known,
            novel: Vec::new(),
            together: BTreeMap::new(),
        }
    }

    fn index(&self, id: Id) -> Option<usize> {
        self.minds.iter().position(|mind| mind.id == id)
    }

    #[cfg(test)]
    pub fn actor(&self, id: Id) -> Option<&Actor> {
        self.actors.iter().find(|actor| actor.id == id)
    }

    /// What has happened since the window last asked.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Whether a visitor is staying over.
    pub fn staying(&self, id: Id) -> bool {
        self.index(id)
            .is_some_and(|index| self.minds[index].staying)
    }

    /// Everyone staying over.
    pub fn guests(&self) -> Vec<Id> {
        self.minds
            .iter()
            .filter(|mind| mind.staying)
            .map(|mind| mind.id)
            .collect()
    }

    /// Ask a visitor to stay over. Whether it will, as suits it and the friendship.
    pub fn ask_to_stay(&mut self, household: &Household, id: Id) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        if self.minds[index].staying {
            return true;
        }
        if !matches!(self.minds[index].presence, Presence::Visiting { .. }) {
            return false;
        }
        let Some(visitor) = household.resident(id) else {
            return false;
        };
        let warmth = household
            .friend_of(id)
            .map_or(0.0, |friend| band(household.bond(friend.id, id).warmth));
        if !visitor.character.agrees_to_stay(warmth) {
            return false;
        }
        self.stay_over(index);
        true
    }

    /// A visitor stays as long as the house is open.
    fn stay_over(&mut self, index: usize) {
        let mind = &mut self.minds[index];
        mind.staying = true;
        mind.presence = Presence::Visiting {
            until: f32::INFINITY,
        };
        self.events.push(Event::StayingOver(mind.id));
    }

    /// Who has spent time together since the house opened, how, and how often.
    pub fn together(&self) -> Vec<(Id, Id, Together, u8)> {
        self.together
            .iter()
            .map(|(&(a, b, how), &times)| (a, b, how, times))
            .collect()
    }

    /// Everyone in the house now, residents and visitors.
    pub fn present(&self) -> Vec<Id> {
        self.minds
            .iter()
            .filter(|mind| mind.presence.in_house() || mind.presence == Presence::Leaving)
            .map(|mind| mind.id)
            .collect()
    }

    pub fn lamps_off(&self) -> &[u16] {
        &self.lamps_off
    }

    /// The finds being worn just now, which are not where they are usually shown.
    pub fn worn(&self) -> Vec<DisplayId> {
        self.worn.iter().map(|(item, _)| item.clone()).collect()
    }

    /// What the owner has asked of someone, the one under way first.
    pub fn queue(&self, id: Id) -> Vec<Act> {
        let Some(mind) = self.index(id).map(|index| &self.minds[index]) else {
            return Vec::new();
        };
        let current = mind
            .plan
            .as_ref()
            .filter(|plan| plan.asked && plan.part == Part::Leads)
            .map(|plan| plan.act.clone());
        current
            .into_iter()
            .chain(mind.queue.iter().cloned())
            .collect()
    }

    /// Ask someone to do something, after whatever else it has been asked. What it was doing of
    /// its own accord it leaves at once.
    pub fn ask(&mut self, id: Id, act: Act, now: f32) -> Asked {
        if self.queue(id).len() >= QUEUE_LIMIT {
            return Asked::Full;
        }
        let Some(index) = self.index(id) else {
            return Asked::Full;
        };
        if !self.minds[index].presence.in_house() {
            return Asked::Full;
        }
        self.minds[index].queue.push_back(act);
        let own_idea = self.minds[index]
            .plan
            .as_ref()
            .is_some_and(|plan| !plan.asked);
        if own_idea {
            self.end(index, now, false);
        }
        self.minds[index].dawdle_until = now;
        Asked::Queued
    }

    /// Take back one thing asked of someone, by its place in the queue.
    pub fn cancel(&mut self, id: Id, position: usize, now: f32) {
        let Some(index) = self.index(id) else { return };
        let leading = self.minds[index]
            .plan
            .as_ref()
            .is_some_and(|plan| plan.asked && plan.part == Part::Leads);
        match (position, leading) {
            (0, true) => self.end(index, now, false),
            (position, true) => {
                if position - 1 < self.minds[index].queue.len() {
                    self.minds[index].queue.remove(position - 1);
                }
            }
            (position, false) => {
                if position < self.minds[index].queue.len() {
                    self.minds[index].queue.remove(position);
                }
            }
        }
    }

    /// A pat from the owner: whatever it was doing is let go, and it answers the pat.
    pub fn pet(&mut self, household: &Household, id: Id, now: f32) {
        let Some(index) = self.index(id) else { return };
        self.drop_everything(index, now);
        let character = household.resident(id).map(|r| r.character.clone());
        let expression = match character.as_ref().map(|c| c.kind) {
            Some(TemperamentKind::Grump) => ExpressionKind::Smug,
            Some(TemperamentKind::Wallflower) => ExpressionKind::Content,
            _ => ExpressionKind::Joy,
        };
        let actor = &mut self.actors[index];
        actor.get_down();
        actor.strike(
            Pose::new(ActionKind::PetReaction, expression).with_cue(Cue::Heart),
            now,
        );
        let mind = &mut self.minds[index];
        mind.handled = Some(Handled::Petted { until: now + 2.2 });
        mind.drives[Drive::Company.index()] *= 0.6;
    }

    /// Picked up by the owner: let go of everything and held.
    pub fn pick_up(&mut self, id: Id, now: f32) {
        let Some(index) = self.index(id) else { return };
        self.drop_everything(index, now);
        let actor = &mut self.actors[index];
        actor.get_down();
        actor.lift = 14.0;
        actor.strike(
            Pose::new(ActionKind::Dragged, ExpressionKind::Startled),
            now,
        );
        self.minds[index].handled = Some(Handled::Held);
    }

    /// Carried across the room to a point on the floor.
    pub fn carry(&mut self, id: Id, point: (f32, f32)) {
        if let Some(index) = self.index(id) {
            self.actors[index].pos = point;
        }
    }

    #[cfg(test)]
    pub fn held(&self) -> Option<Id> {
        self.minds
            .iter()
            .find(|mind| mind.handled == Some(Handled::Held))
            .map(|mind| mind.id)
    }

    /// Set down on the open tile nearest where it was let go.
    pub fn put_down(&mut self, house: &House, id: Id, now: f32) {
        let Some(index) = self.index(id) else { return };
        let floor = Floor::of(house);
        let tile = floor
            .nearest_open(Floor::tile_of(self.actors[index].pos))
            .unwrap_or((0, 0));
        let actor = &mut self.actors[index];
        actor.pos = (tile.0 as f32 + 0.5, tile.1 as f32 + 0.5);
        actor.lift = 0.0;
        actor.strike(Pose::new(ActionKind::Landing, ExpressionKind::Neutral), now);
        self.minds[index].handled = Some(Handled::Landing { until: now + 0.6 });
    }

    /// Everyone stops where they are, gets down off anything, and waits while the room is
    /// arranged round them. A visitor still to come waits too.
    pub fn pause(&mut self, household: &Household, house: &House, now: f32) {
        self.paused = true;
        for index in 0..self.minds.len() {
            self.end(index, now, false);
            self.minds[index].handled = None;
            if self.actors[index].hidden {
                continue;
            }
            let face = household
                .resident(self.minds[index].id)
                .map_or(ExpressionKind::Neutral, |r| r.character.idle_face());
            let actor = &mut self.actors[index];
            actor.get_down();
            actor.stop();
            actor.lift = 0.0;
            actor.strike(Pose::idle(face), now);
        }
        self.make_room(house);
    }

    /// Back to life, minus anything asked for that the room no longer has. Whatever has been put
    /// on show since the room was last arranged is new, and an inquisitive resident will go and
    /// look.
    pub fn resume(&mut self, house: &House, snapshot: &HomeSnapshot, now: f32) {
        self.paused = false;
        self.make_room(house);
        for mind in &mut self.minds {
            mind.queue.retain(|act| still_there(act, house, snapshot));
            mind.dawdle_until = mind.dawdle_until.max(now + 0.3);
        }
        let shown: Vec<DisplayId> = house.shown.iter().map(|shown| shown.item.clone()).collect();
        self.novel = shown
            .iter()
            .filter(|item| !self.known.contains(item))
            .cloned()
            .collect();
        self.known = shown;
        self.lamps_off.retain(|uid| house.piece(*uid).is_some());
    }

    #[cfg(test)]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Anyone standing where a piece now stands steps to the nearest open floor.
    pub fn make_room(&mut self, house: &House) {
        let floor = Floor::of(house);
        let mut taken: Vec<(i32, i32)> = Vec::new();
        for actor in &mut self.actors {
            if actor.on_piece.is_some() || actor.hidden {
                continue;
            }
            let tile = Floor::tile_of(actor.pos);
            if floor.open(tile.0, tile.1) && !taken.contains(&tile) {
                taken.push(tile);
                continue;
            }
            if let Some(open) = floor.nearest_open(tile) {
                actor.stop();
                actor.pos = (open.0 as f32 + 0.5, open.1 as f32 + 0.5);
                taken.push(open);
            }
        }
    }

    /// What someone is doing, as the room's status line puts it.
    pub fn doing(
        &self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        id: Id,
    ) -> String {
        let Some(index) = self.index(id) else {
            return String::new();
        };
        let mind = &self.minds[index];
        let name = household
            .resident(id)
            .map_or("Someone", |r| r.name.as_str());
        match (&mind.presence, &mind.handled, &mind.plan) {
            (Presence::Leaving, ..) => format!("{name} is on the way home."),
            (Presence::Expected { .. } | Presence::Gone, ..) => String::new(),
            (_, Some(Handled::Held), _) => format!("{name} is being carried."),
            (_, Some(Handled::Petted { .. }), _) => format!("{name} is enjoying a pat."),
            (_, _, Some(plan)) => match plan.part {
                Part::Joins(with) => {
                    let leader = household
                        .resident(with)
                        .map_or("someone", |r| r.name.as_str());
                    format!("{name} is with {leader}.")
                }
                Part::Leads => {
                    format!("{name} is {}.", plan.act.doing(household, house, snapshot))
                }
            },
            _ if self.paused => format!("{name} is waiting while the room is arranged."),
            _ => format!("{name} is pottering about."),
        }
    }

    /// Everything that happens in `dt` seconds up to `now`, with `likings` what the household
    /// has come to like.
    pub fn tick(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        likings: &[Liking],
        now: f32,
        dt: f32,
    ) {
        if self.paused {
            return;
        }
        let dt = dt.clamp(0.0, 0.25);
        let floor = Floor::of(house);
        for index in 0..self.minds.len() {
            let Some(resident) = household.resident(self.minds[index].id) else {
                continue;
            };
            match self.minds[index].presence {
                Presence::Expected { at } if now >= at => {
                    self.knock(household, house, &floor, index, now);
                }
                Presence::Expected { .. } | Presence::Gone => continue,
                Presence::Leaving => {
                    self.actors[index].advance(dt);
                    if !self.actors[index].walking() {
                        self.actors[index].hidden = true;
                        self.minds[index].presence = Presence::Gone;
                        let id = self.minds[index].id;
                        // Something to remember the visit by, as suits the friend and the
                        // friendship.
                        let warmth = household
                            .friend_of(id)
                            .map_or(0.0, |friend| band(household.bond(friend.id, id).warmth));
                        let character = &resident.character;
                        let gives = keepsakes::gives(character, warmth);
                        let gift = (self.minds[index].dice.next() < gives)
                            .then(|| keepsakes::gift(character));
                        self.events.push(Event::Left(id, gift));
                    }
                    continue;
                }
                Presence::Visiting { until }
                    if now >= until
                        && self.minds[index].handled.is_none()
                        && !self.minds[index].plan.as_ref().is_some_and(|p| p.asked) =>
                {
                    // A close friend may ask to stay over, if nobody else is.
                    let id = self.minds[index].id;
                    let warmth = household
                        .friend_of(id)
                        .map_or(0.0, |friend| band(household.bond(friend.id, id).warmth));
                    let asks = resident.character.asks_to_stay(warmth);
                    let nobody_yet = !self.minds.iter().any(|mind| mind.staying);
                    if nobody_yet && self.minds[index].dice.next() < asks {
                        self.stay_over(index);
                    } else {
                        self.go_home(house, &floor, index, now);
                        continue;
                    }
                }
                _ => {}
            }
            let rates = resident.character.drive_rates();
            let mind = &mut self.minds[index];
            for (drive, rate) in mind.drives.iter_mut().zip(rates) {
                *drive = (*drive + rate * dt / 60.0).min(1.0);
            }
            match mind.handled {
                Some(Handled::Held) => continue,
                Some(Handled::Petted { until } | Handled::Landing { until }) if now < until => {
                    continue;
                }
                Some(_) => {
                    mind.handled = None;
                    mind.dawdle_until = now + 0.5;
                    let face = resident.character.idle_face();
                    self.actors[index].strike(Pose::idle(face), now);
                }
                None => {}
            }
            let actor = &mut self.actors[index];
            actor.advance(dt);
            let room = house.room_of_point(actor.pos);
            let mind = &mut self.minds[index];
            if room != mind.room {
                mind.room = room;
                mind.entered = now;
            }
            let roams = household
                .resident(mind.id)
                .map_or(0.5, |resident| resident.character.roams());
            let palls = ROOM_RESTLESS.0 + (ROOM_RESTLESS.1 - ROOM_RESTLESS.0) * roams;
            mind.restless = ((now - mind.entered) / palls).clamp(0.0, 1.0);
            let actor = &mut self.actors[index];
            // Nobody stands about on the furniture: off a piece, the open floor is a step away.
            let tile = Floor::tile_of(actor.pos);
            let inside = house.room_of_point(actor.pos).is_some();
            if actor.on_piece.is_none()
                && !actor.walking()
                && inside
                && !floor.open(tile.0, tile.1)
                && let Some(open) = floor.nearest_open(tile)
            {
                actor.walk([(open.0 as f32 + 0.5, open.1 as f32 + 0.5)]);
            }
            self.step(household, house, snapshot, likings, &floor, index, now);
        }
    }

    /// A visitor at the door: in it comes, and the household answers in character.
    fn knock(
        &mut self,
        household: &Household,
        house: &House,
        floor: &Floor,
        index: usize,
        now: f32,
    ) {
        let (outside, inside) = door(house, floor);
        let mind = &mut self.minds[index];
        let stay = mind.dice.between(150.0, 210.0);
        mind.presence = Presence::Visiting { until: now + stay };
        mind.dawdle_until = now + 3.0;
        let visitor = mind.id;
        let actor = &mut self.actors[index];
        actor.hidden = false;
        actor.pos = outside;
        actor.walk([(inside.0 as f32 + 0.5, inside.1 as f32 + 0.5)]);
        self.events.push(Event::Arrived(visitor));
        let snapshot = &household.snapshot;
        for resident in &household.residents {
            let Some(r) = self.index(resident.id) else {
                continue;
            };
            let busy = self.minds[r].handled.is_some()
                || self.minds[r].plan.as_ref().is_some_and(|plan| {
                    plan.asked || matches!(plan.act, Act::Sleep(_) | Act::InviteLittle(..))
                });
            if busy {
                continue;
            }
            let character = &resident.character;
            let bond = household.bond(resident.id, visitor);
            let answer = if character.kind == TemperamentKind::Sweetheart
                || bond.close()
                || character.axes.social > 0.7
            {
                Some(Act::Greet(visitor))
            } else if character.kind == TemperamentKind::Grump && bond.warmth < Band::Medium {
                Some(Act::GrumbleAt(visitor))
            } else if character.keeps_apart() {
                // Off to the seat furthest from the door, or failing one, the far corner.
                let seats = self.free_seats(house);
                let far = seats.iter().max_by(|a, b| {
                    let d = |s: &Seat| (s.at.0 - outside.0).powi(2) + (s.at.1 - outside.1).powi(2);
                    d(a).total_cmp(&d(b))
                });
                far.map(|seat| Act::CurlUp(seat.piece))
                    .filter(|act| {
                        act.piece()
                            .and_then(|uid| house.piece(uid))
                            .and_then(|placed| catalog::piece(&placed.piece))
                            .is_some_and(|piece| piece.has(Use::Nap))
                    })
                    .or_else(|| far.map(|seat| Act::Sit(seat.piece)))
            } else {
                None
            };
            if let Some(act) = answer {
                self.end(r, now, false);
                self.begin(house, snapshot, floor, r, act, false, now);
            }
        }
    }

    /// Time for a visitor to go: whatever it was doing it leaves, and out it goes the way it came.
    fn go_home(&mut self, house: &House, floor: &Floor, index: usize, now: f32) {
        self.end(index, now, false);
        let (outside, inside) = door(house, floor);
        let actor = &mut self.actors[index];
        let mut route = floor.route(actor.pos, inside).unwrap_or_default();
        route.push(outside);
        actor.walk(route);
        self.minds[index].presence = Presence::Leaving;
        self.minds[index].queue.clear();
    }

    /// Move one person's plan along, or give it a new one.
    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        likings: &[Liking],
        floor: &Floor,
        index: usize,
        now: f32,
    ) {
        let Some(plan) = self.minds[index].plan.clone() else {
            if now < self.minds[index].dawdle_until {
                return;
            }
            let next = match self.minds[index].queue.pop_front() {
                Some(act) => Some((act, true)),
                None => self
                    .choose(household, house, snapshot, likings, index)
                    .map(|act| (act, false)),
            };
            if let Some((act, asked)) = next {
                self.begin(house, snapshot, floor, index, act, asked, now);
            } else {
                self.minds[index].dawdle_until = now + 2.0;
            }
            return;
        };
        if let Part::Joins(leader) = plan.part {
            let led = self.index(leader).is_some_and(|l| {
                self.minds[l].plan.as_ref().is_some_and(|p| {
                    p.part == Part::Leads && p.act.partner() == Some(self.minds[index].id)
                })
            });
            if !led {
                self.end(index, now, false);
            } else if plan.stage == Stage::Going && !self.actors[index].walking() {
                if let Some(l) = self.index(leader) {
                    let there = self.actors[l].pos;
                    self.actors[index].face_towards(there);
                }
                self.set_stage(index, Stage::Waiting { since: now });
            }
            return;
        }
        match plan.stage {
            Stage::Going => {
                if self.actors[index].walking() {
                    return;
                }
                let partner = plan.act.partner();
                let partner_here = partner.is_none_or(|other| {
                    self.index(other).is_some_and(|o| {
                        !self.actors[o].walking()
                            && self.minds[o]
                                .plan
                                .as_ref()
                                .is_some_and(|p| p.part == Part::Joins(self.minds[index].id))
                    })
                });
                if partner_here {
                    self.arrive(household, house, snapshot, index, &plan, now);
                } else {
                    self.set_stage(index, Stage::Waiting { since: now });
                }
            }
            Stage::Waiting { since } => {
                let partner = plan.act.partner().and_then(|other| self.index(other));
                let ready = partner.is_none_or(|o| !self.actors[o].walking());
                if ready || now - since > 6.0 {
                    self.arrive(household, house, snapshot, index, &plan, now);
                }
            }
            Stage::Prelude { until } => {
                if now >= until {
                    self.perform(household, house, snapshot, index, &plan, now);
                }
            }
            Stage::Doing { until } => {
                if plan.part == Part::Leads && now >= until {
                    self.end(index, now, true);
                }
            }
        }
    }

    fn set_stage(&mut self, index: usize, stage: Stage) {
        if let Some(plan) = &mut self.minds[index].plan {
            plan.stage = stage;
        }
    }

    /// Start on an act: work out where it is done from, and set off there; bring whoever it is
    /// done with.
    #[allow(clippy::too_many_arguments)]
    fn begin(
        &mut self,
        house: &House,
        snapshot: &HomeSnapshot,
        floor: &Floor,
        index: usize,
        act: Act,
        asked: bool,
        now: f32,
    ) {
        let id = self.minds[index].id;
        if !still_there(&act, house, snapshot) || self.worn_by_another(&act, id) {
            self.minds[index].dawdle_until = now + 0.5;
            return;
        }
        let partner = act
            .partner()
            .and_then(|other| self.index(other))
            .filter(|&o| o != index && self.minds[o].presence.in_house());
        if act.partner().is_some() && partner.is_none() {
            self.minds[index].dawdle_until = now + 0.5;
            return;
        }
        // Somebody asleep because they were asked to be, or being handled, is left be.
        if let Some(o) = partner
            && (self.minds[o].handled.is_some()
                || self.minds[o]
                    .plan
                    .as_ref()
                    .is_some_and(|p| p.asked && p.part == Part::Leads))
        {
            self.minds[index].dawdle_until = now + if asked { 1.0 } else { 0.5 };
            if !asked {
                return;
            }
        }
        let seats = self.free_seats(house);
        let spot = self.spot_for(house, floor, index, &act, &seats);
        let Some((tile, seat, face)) = spot else {
            self.minds[index].dawdle_until = now + 1.0;
            return;
        };
        let actor = &mut self.actors[index];
        let route = floor
            .route(actor.pos, tile)
            .unwrap_or_else(|| vec![(tile.0 as f32 + 0.5, tile.1 as f32 + 0.5)]);
        actor.walk(route);
        self.minds[index].plan = Some(Plan {
            act: act.clone(),
            part: Part::Leads,
            asked,
            stage: Stage::Going,
            seat,
            face,
        });
        if let Some(o) = partner
            && self.minds[o].handled.is_none()
            && !self.minds[o]
                .plan
                .as_ref()
                .is_some_and(|p| p.asked && p.part == Part::Leads)
        {
            self.end(o, now, false);
            let mut free = seats.clone();
            if let Some(mine) = seat {
                free.retain(|other| *other != mine);
            }
            let joined = self.join_spot(house, floor, &act, tile, seat, &free);
            let (their_tile, their_seat) = joined.unwrap_or((tile, None));
            let other = &mut self.actors[o];
            let route = floor
                .route(other.pos, their_tile)
                .unwrap_or_else(|| vec![(their_tile.0 as f32 + 0.5, their_tile.1 as f32 + 0.5)]);
            other.walk(route);
            self.minds[o].plan = Some(Plan {
                act: act.clone(),
                part: Part::Joins(id),
                asked: false,
                stage: Stage::Going,
                seat: their_seat,
                face: Some((tile.0 as f32 + 0.5, tile.1 as f32 + 0.5)),
            });
        }
    }

    /// Whether the find an act is about is being worn by somebody else just now.
    fn worn_by_another(&self, act: &Act, id: Id) -> bool {
        act.item()
            .is_some_and(|item| self.worn.iter().any(|(worn, by)| worn == item && *by != id))
    }

    /// Every seat and lying place in the room not already taken.
    fn free_seats(&self, house: &House) -> Vec<Seat> {
        let mut seats = Vec::new();
        for placed in &house.pieces {
            let Some(piece) = catalog::piece(&placed.piece) else {
                continue;
            };
            let (fx, fy) = catalog::front(placed.turn);
            for point in seat_points(placed, piece) {
                if self.seat_taken(placed.uid, point).is_none() {
                    seats.push(Seat {
                        piece: placed.uid,
                        at: point,
                        lift: piece.lift as f32,
                        facing_right: fx - fy > 0,
                    });
                }
            }
        }
        seats
    }

    /// Who has a place on a piece, if anyone.
    fn seat_taken(&self, piece: u16, point: (f32, f32)) -> Option<Id> {
        self.minds
            .iter()
            .find(|mind| {
                mind.plan
                    .as_ref()
                    .and_then(|plan| plan.seat)
                    .is_some_and(|seat| seat.piece == piece && seat.at == point)
            })
            .map(|mind| mind.id)
    }

    /// Who is on a piece, if anyone is.
    fn on_piece(&self, piece: u16) -> Option<Id> {
        self.minds
            .iter()
            .find(|mind| {
                mind.plan
                    .as_ref()
                    .and_then(|plan| plan.seat)
                    .is_some_and(|seat| seat.piece == piece)
            })
            .map(|mind| mind.id)
    }

    /// Where an act is done from: the tile to walk to, the seat to settle on there if any, and
    /// what to look at.
    #[allow(clippy::type_complexity)]
    fn spot_for(
        &self,
        house: &House,
        floor: &Floor,
        index: usize,
        act: &Act,
        seats: &[Seat],
    ) -> Option<((i32, i32), Option<Seat>, Option<(f32, f32)>)> {
        let me = self.actors[index].pos;
        let nearest = |tiles: Vec<(i32, i32)>| {
            tiles.into_iter().min_by(|a, b| {
                let d = |t: &(i32, i32)| {
                    (t.0 as f32 + 0.5 - me.0).powi(2) + (t.1 as f32 + 0.5 - me.1).powi(2)
                };
                d(a).total_cmp(&d(b))
            })
        };
        if let Act::Sprawl(uid) = act {
            // Out on the rug itself, which is walked over: the open tile of it nearest its middle.
            let placed = house.piece(*uid)?;
            let footprint = room::footprint(placed);
            let (cx, cy) = footprint.centre();
            let tile = footprint
                .tiles()
                .filter(|(x, y)| floor.open(i32::from(*x), i32::from(*y)))
                .min_by(|a, b| {
                    let d = |t: &(u8, u8)| {
                        (f32::from(t.0) + 0.5 - cx).powi(2) + (f32::from(t.1) + 0.5 - cy).powi(2)
                    };
                    d(a).total_cmp(&d(b))
                })?;
            return Some(((i32::from(tile.0), i32::from(tile.1)), None, None));
        }
        if let Some(uid) = act.piece() {
            let placed = house.piece(uid)?;
            let footprint = room::footprint(placed);
            let around = floor.beside(
                placed.x,
                placed.y,
                footprint.w,
                footprint.d,
                catalog::front(placed.turn),
            );
            let tile = *around.first()?;
            if act.settles() {
                let seat = seats.iter().find(|seat| seat.piece == uid).copied()?;
                return Some((tile, Some(seat), None));
            }
            return Some((tile, None, Some(footprint.centre())));
        }
        if let Some(item) = act.item() {
            let (around, look) = item_spot(house, floor, item)?;
            return Some((nearest(around)?, None, Some(look)));
        }
        match act {
            Act::GoTo(x, y) | Act::Wander(x, y) => {
                let tile = (i32::from(*x), i32::from(*y));
                floor.open(tile.0, tile.1).then_some((tile, None, None))
            }
            Act::SitTogether(other) => {
                // Two seats on one piece if there are any; otherwise the seat nearest each.
                let pair = seats.iter().find_map(|a| {
                    seats
                        .iter()
                        .find(|b| b.piece == a.piece && b.at != a.at)
                        .map(|_| *a)
                });
                let seat = pair.or_else(|| seats.first().copied())?;
                let placed = house.piece(seat.piece)?;
                let footprint = room::footprint(placed);
                let tile = *floor
                    .beside(
                        placed.x,
                        placed.y,
                        footprint.w,
                        footprint.d,
                        catalog::front(placed.turn),
                    )
                    .first()?;
                self.index(*other)?;
                Some((tile, Some(seat), None))
            }
            Act::Greet(other)
            | Act::PlayTogether(other)
            | Act::Tease(other)
            | Act::Comfort(other)
            | Act::Hug(other)
            | Act::GrumbleAt(other) => {
                let o = self.index(*other)?;
                // Someone still coming in is met at the door.
                let theirs = self.actors[o].destination();
                let there = Floor::tile_of(theirs);
                let there = floor.nearest_open(there)?;
                let around = floor.beside(there.0 as u8, there.1 as u8, 1, 1, (0, 1));
                Some((nearest(around).unwrap_or(there), None, Some(theirs)))
            }
            _ => None,
        }
    }

    /// Where whoever joins an act goes: beside the leader, or to the other seat.
    fn join_spot(
        &self,
        house: &House,
        floor: &Floor,
        act: &Act,
        leader: (i32, i32),
        leader_seat: Option<Seat>,
        seats: &[Seat],
    ) -> Option<((i32, i32), Option<Seat>)> {
        let settles = matches!(act, Act::SitTogether(_) | Act::InviteLittle(..));
        if settles {
            let near = |uid: u16| {
                let placed = house.piece(uid)?;
                let footprint = room::footprint(placed);
                floor
                    .beside(
                        placed.x,
                        placed.y,
                        footprint.w,
                        footprint.d,
                        catalog::front(placed.turn),
                    )
                    .first()
                    .copied()
            };
            // The same piece first: the other half of the sofa, the foot of the bed.
            let piece = leader_seat.map(|seat| seat.piece).or(act.piece());
            let seat = seats
                .iter()
                .find(|seat| Some(seat.piece) == piece)
                .or_else(|| seats.first())
                .copied();
            if let Some(seat) = seat {
                return Some((near(seat.piece).unwrap_or(leader), Some(seat)));
            }
        }
        let around = floor.beside(leader.0 as u8, leader.1 as u8, 1, 1, (1, 0));
        Some((*around.first().unwrap_or(&leader), None))
    }

    /// There: settle on the seat, turn to what it is about, and, after any little flourish its
    /// habits ask for, get on with it.
    fn arrive(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        index: usize,
        plan: &Plan,
        now: f32,
    ) {
        let resident = household.resident(self.minds[index].id);
        let actor = &mut self.actors[index];
        if let Some(seat) = plan.seat {
            actor.settle_on(seat.piece, seat.at, seat.lift, seat.facing_right);
        }
        if let Some(face) = plan.face {
            actor.face_towards(face);
        }
        let lying = matches!(
            plan.act,
            Act::Nap(_) | Act::Sleep(_) | Act::CurlUp(_) | Act::InviteLittle(..) | Act::Sprawl(_)
        );
        let prelude = resident.and_then(|resident| {
            if !lying || self.reduce_motion {
                return None;
            }
            if resident.character.has_habit(Habit::CirclesBeforeNaps) {
                Some(Pose::idle(ExpressionKind::Content).spinning())
            } else if resident.character.has_habit(Habit::StretchesBeforeNaps) {
                Some(Pose::new(Gesture::Stretch, ExpressionKind::Sleepy))
            } else {
                None
            }
        });
        match prelude {
            Some(pose) => {
                self.actors[index].strike(pose, now);
                self.set_stage(index, Stage::Prelude { until: now + 1.1 });
            }
            None => self.perform(household, house, snapshot, index, plan, now),
        }
        // Whoever joins it gets on with their part as the leader does.
        if plan.part == Part::Leads
            && let Some(o) = plan.act.partner().and_then(|other| self.index(other))
            && let Some(theirs) = self.minds[o].plan.clone()
            && theirs.part == Part::Joins(self.minds[index].id)
        {
            let actor = &mut self.actors[o];
            if let Some(seat) = theirs.seat {
                actor.settle_on(seat.piece, seat.at, seat.lift, seat.facing_right);
            } else {
                let leader = self.actors[index].pos;
                self.actors[o].face_towards(leader);
                if plan.seat.is_none() {
                    let theirs = self.actors[o].pos;
                    self.actors[index].face_towards(theirs);
                }
            }
            if let Some(face) = plan
                .face
                .filter(|_| plan.act.item().is_some() || plan.act.piece().is_some())
            {
                self.actors[index].face_towards(face);
            }
            self.perform(household, house, snapshot, o, &theirs, now);
        }
    }

    /// Strike the act's pose and hold it as long as the act lasts. A lamp is switched and a find
    /// put on as the act begins.
    fn perform(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        index: usize,
        plan: &Plan,
        now: f32,
    ) {
        let Some(resident) = household.resident(self.minds[index].id) else {
            return;
        };
        let id = resident.id;
        match &plan.act {
            Act::SwitchLamp(uid) if plan.part == Part::Leads => {
                match self.lamps_off.iter().position(|off| off == uid) {
                    Some(at) => {
                        self.lamps_off.remove(at);
                    }
                    None => self.lamps_off.push(*uid),
                }
            }
            Act::TryOn(item) if plan.part == Part::Leads => {
                if let Some(dress) = snapshot.item(item).and_then(wearable) {
                    self.worn.push((item.clone(), id));
                    self.actors[index].wear(Some(dress));
                }
            }
            act => {
                if let Some(item) = act.item()
                    && !self.minds[index].seen.contains(item)
                {
                    self.minds[index].seen.push(item.clone());
                }
                if let Act::Browse(uid) = act {
                    for item in shown_on(house, *uid) {
                        if !self.minds[index].seen.contains(&item) {
                            self.minds[index].seen.push(item);
                        }
                    }
                }
                if let Some(item) = act.item() {
                    self.novel.retain(|novel| novel != item);
                }
            }
        }
        let character = &resident.character;
        let dice = &mut self.minds[index].dice;
        let settled = character.settled_face();
        let precious = |item: &DisplayId| snapshot.item(item).is_some_and(is_precious);
        let (pose, seconds) = match (&plan.part, &plan.act) {
            (Part::Joins(leader), act) => {
                let bond = household.bond(resident.id, *leader);
                let warm = bond.warmth >= Band::Medium || household.family(resident.id, *leader);
                let pose = match act {
                    Act::Tease(_) if character.axes.feistiness > 0.55 => {
                        Pose::idle(ExpressionKind::Grumpy).with_cue(Cue::Grumble)
                    }
                    Act::Tease(_) => Pose::idle(ExpressionKind::Worried).with_cue(Cue::Bang),
                    Act::Hug(_) | Act::Comfort(_) => {
                        Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart)
                    }
                    Act::Greet(_) if warm => Pose::new(ActionKind::Greet, ExpressionKind::Joy),
                    Act::Greet(_) => Pose::idle(character.idle_face()),
                    Act::PlayTogether(_) | Act::PlayWith(..) => {
                        Pose::new(ActionKind::SocialPlay, ExpressionKind::Joy).with_cue(Cue::Note)
                    }
                    Act::ShowTo(..) if character.kind == TemperamentKind::Grump => {
                        Pose::new(Gesture::Watch, ExpressionKind::Bored)
                    }
                    Act::ShowTo(..) => Pose::new(Gesture::Watch, ExpressionKind::Curious),
                    Act::Share(..) => Pose::new(ActionKind::Eat, ExpressionKind::Content),
                    Act::SitTogether(_) => Pose::new(ActionKind::Perch, settled),
                    Act::InviteLittle(..) => {
                        Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep)
                    }
                    _ => Pose::idle(character.idle_face()),
                };
                (pose, f32::MAX)
            }
            (Part::Leads, act) => match act {
                Act::GoTo(..) => (Pose::idle(character.idle_face()), 1.0),
                Act::Wander(..) => (
                    Pose::new(Gesture::Watch, character.walk_face()),
                    dice.between(1.5, 3.5),
                ),
                Act::Sit(_) => (
                    Pose::new(ActionKind::Perch, settled),
                    dice.between(6.0, 10.0),
                ),
                Act::Relax(_) => (
                    Pose::new(ActionKind::Perch, ExpressionKind::Content),
                    dice.between(12.0, 18.0),
                ),
                Act::SitAt(_) => (
                    Pose::new(ActionKind::Perch, settled),
                    dice.between(8.0, 12.0),
                ),
                Act::Draw(..) => (
                    Pose::new(ActionKind::Perch, ExpressionKind::Focused).with_cue(Cue::Thought),
                    dice.between(10.0, 14.0),
                ),
                Act::Nap(_) | Act::CurlUp(_) => {
                    let long = if character.kind == TemperamentKind::Lazybones {
                        1.5
                    } else {
                        1.0
                    };
                    (
                        Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep),
                        dice.between(12.0, 18.0) * long,
                    )
                }
                Act::Sprawl(_) => (
                    Pose::new(ActionKind::Sleep, ExpressionKind::Content).with_cue(Cue::Sleep),
                    dice.between(10.0, 16.0),
                ),
                Act::Sleep(_) | Act::InviteLittle(..) => (
                    Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep),
                    dice.between(20.0, 28.0),
                ),
                Act::Bounce(_) => (
                    Pose::new(Gesture::Cheer, ExpressionKind::Joy)
                        .spinning()
                        .with_cue(Cue::Note),
                    4.0,
                ),
                Act::SwitchLamp(_) => (Pose::new(Gesture::Reach, ExpressionKind::Curious), 1.4),
                Act::Tend(_) => (
                    Pose::new(Gesture::Reach, ExpressionKind::Content).with_cue(Cue::Heart),
                    4.0,
                ),
                Act::Browse(_) => (
                    Pose::new(ActionKind::InspectScreen, ExpressionKind::Curious)
                        .with_cue(Cue::Question),
                    dice.between(5.0, 7.0),
                ),
                Act::Play(_) | Act::PlayWithFind(_) => (
                    Pose::new(ActionKind::SoloPlay, ExpressionKind::Joy).with_cue(Cue::Note),
                    dice.between(6.0, 9.0),
                ),
                Act::PlayWith(..) | Act::PlayTogether(_) => (
                    Pose::new(ActionKind::SocialPlay, ExpressionKind::Joy).with_cue(Cue::Note),
                    dice.between(6.0, 8.0),
                ),
                Act::ShowOffToy(_) => (
                    Pose::new(Gesture::Cheer, ExpressionKind::Smug).with_cue(Cue::Sparkle),
                    3.5,
                ),
                Act::Snack(_) | Act::Share(..) => {
                    let face = if resident
                        .traveler
                        .character
                        .trait_ids
                        .contains(&Trait::FoodMotivated)
                    {
                        ExpressionKind::Joy
                    } else {
                        ExpressionKind::Content
                    };
                    (Pose::new(ActionKind::Eat, face), 5.0)
                }
                Act::Inspect(item) => {
                    let face = if character.studies() {
                        ExpressionKind::Focused
                    } else {
                        ExpressionKind::Curious
                    };
                    let long = if character.studies() && precious(item) {
                        1.5
                    } else {
                        1.0
                    };
                    (
                        Pose::new(ActionKind::InspectScreen, face).with_cue(Cue::Question),
                        dice.between(4.0, 6.0) * long,
                    )
                }
                Act::Remember(_) => (
                    Pose::new(Gesture::Watch, ExpressionKind::Affectionate).with_cue(Cue::Thought),
                    6.0,
                ),
                Act::ShowTo(..) => (
                    Pose::new(ActionKind::PresentDiscovery, ExpressionKind::Smug)
                        .with_cue(Cue::Sparkle),
                    5.0,
                ),
                Act::FussWith(_) => (Pose::new(Gesture::Reach, ExpressionKind::Smug), 3.5),
                Act::TryOn(_) => {
                    let face = if character.shows_off() {
                        ExpressionKind::Smug
                    } else {
                        ExpressionKind::Joy
                    };
                    (
                        Pose::new(Gesture::Bop, face).with_cue(Cue::Sparkle),
                        dice.between(7.0, 10.0),
                    )
                }
                Act::Greet(_) => (Pose::new(ActionKind::Greet, ExpressionKind::Joy), 2.5),
                Act::SitTogether(_) => (
                    Pose::new(ActionKind::Perch, ExpressionKind::Content),
                    dice.between(10.0, 14.0),
                ),
                Act::Tease(_) => (
                    Pose::new(Gesture::Crouch, ExpressionKind::Smug).with_cue(Cue::Bang),
                    3.0,
                ),
                Act::Comfort(_) => (
                    Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart),
                    4.0,
                ),
                Act::Hug(_) => (
                    Pose::idle(ExpressionKind::Affectionate).with_cue(Cue::Heart),
                    3.5,
                ),
                Act::GrumbleAt(_) => (
                    Pose::idle(ExpressionKind::Grumpy).with_cue(Cue::Grumble),
                    2.5,
                ),
            },
        };
        let pose = if self.reduce_motion {
            pose.held()
        } else {
            pose
        };
        self.actors[index].strike(pose, now);
        self.set_stage(
            index,
            Stage::Doing {
                until: now + seconds,
            },
        );
    }

    /// Done, or let go: drives settle if it was done, it gets down off anything, puts back
    /// anything it was wearing, and whoever was with it is let go too. A resident that finished
    /// with something in its home has used it once more. The next thing asked of it, if
    /// anything, is next.
    fn end(&mut self, index: usize, now: f32, finished: bool) {
        let Some(plan) = self.minds[index].plan.take() else {
            return;
        };
        let id = self.minds[index].id;
        let mind = &mut self.minds[index];
        if finished {
            let drive = plan.act.drive().index();
            mind.drives[drive] = (mind.drives[drive] - 0.7).max(0.0);
            if !plan.asked {
                mind.last = Some(plan.act.clone());
            }
            if let Act::Draw(_, of) = plan.act {
                if !plan.asked {
                    mind.drew = true;
                }
                self.events.push(Event::Drew(id, of));
            }
            if plan.part == Part::Leads
                && let Some((how, other)) = plan.act.together().zip(plan.act.partner())
            {
                let key = (id.min(other), id.max(other), how);
                let times = self.together.entry(key).or_insert(0);
                *times = (*times + 1).min(MAX_TOGETHER);
            }
            let mind = &mut self.minds[index];
            if mind.presence == Presence::Home && plan.part == Part::Leads {
                let liked = match (&plan.act, plan.act.piece(), plan.act.item()) {
                    (Act::SwitchLamp(_) | Act::Browse(_), _, _) => None,
                    (_, Some(uid), _) => Some(Used::Piece(uid)),
                    (_, _, Some(item)) => Some(Used::Shown(item.clone())),
                    _ => None,
                };
                if let Some(liked) = liked {
                    self.events.push(Event::Used(id, liked));
                }
            }
        }
        let mind = &mut self.minds[index];
        mind.dawdle_until = now
            + if plan.asked {
                0.4
            } else {
                mind.dice.between(1.5, 4.0)
            };
        if let Act::TryOn(item) = &plan.act {
            self.worn.retain(|(worn, by)| !(worn == item && *by == id));
            self.actors[index].wear(None);
        }
        let actor = &mut self.actors[index];
        actor.stop();
        if actor.on_piece.is_some() {
            actor.get_down();
        }
        actor.strike(Pose::idle(ExpressionKind::Content), now);
        if plan.part == Part::Leads
            && let Some(o) = plan.act.partner().and_then(|other| self.index(other))
            && self.minds[o]
                .plan
                .as_ref()
                .is_some_and(|p| p.part == Part::Joins(id))
        {
            self.end(o, now, finished);
        }
    }

    fn drop_everything(&mut self, index: usize, now: f32) {
        self.minds[index].queue.clear();
        self.end(index, now, false);
        // Whoever it had joined carries on without it.
        let id = self.minds[index].id;
        for other in 0..self.minds.len() {
            if self.minds[other]
                .plan
                .as_ref()
                .is_some_and(|plan| plan.part == Part::Leads && plan.act.partner() == Some(id))
            {
                // Nothing to wait for now.
                if let Some(plan) = &mut self.minds[other].plan
                    && matches!(plan.stage, Stage::Going | Stage::Waiting { .. })
                {
                    plan.stage = Stage::Waiting { since: f32::MIN };
                }
            }
        }
    }

    /// What someone would like to do next, of its own accord.
    fn choose(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        likings: &[Liking],
        index: usize,
    ) -> Option<Act> {
        let id = self.minds[index].id;
        let resident = household.resident(id)?;
        let character = &resident.character;
        let visiting = household.is_visitor(id);
        let seats = self.free_seats(house);
        let others: Vec<Id> = self
            .minds
            .iter()
            .filter(|mind| {
                mind.id != id
                    && mind.presence.in_house()
                    && mind.handled.is_none()
                    && !mind.plan.as_ref().is_some_and(|p| {
                        p.asked || matches!(p.act, Act::Sleep(_) | Act::Nap(_) | Act::Sprawl(_))
                    })
            })
            .map(|mind| mind.id)
            .collect();
        let visitors_here = self
            .minds
            .iter()
            .any(|mind| mind.id != id && household.is_visitor(mind.id) && mind.presence.in_house());
        let loved = favourites(likings, house, id);
        let favourite = |thing: Liked| loved.iter().any(|(_, liked)| *liked == thing);
        let piece_favourite = |uid: u16| {
            if house.liked(uid).is_some_and(&favourite) {
                1.8
            } else {
                1.0
            }
        };
        let me = self.actors[index].pos;
        let mut options: Vec<(Act, f32)> = Vec::new();
        let apart = |at: (f32, f32)| {
            self.actors
                .iter()
                .filter(|actor| actor.id != id && !actor.hidden)
                .map(|actor| ((actor.pos.0 - at.0).powi(2) + (actor.pos.1 - at.1).powi(2)).sqrt())
                .fold(8.0_f32, f32::min)
        };
        let kind = character.kind;
        let lazy = kind == TemperamentKind::Lazybones;
        for placed in &house.pieces {
            let Some(piece) = catalog::piece(&placed.piece) else {
                continue;
            };
            let uid = placed.uid;
            let free = seats.iter().any(|seat| seat.piece == uid);
            let centre = room::footprint(placed).centre();
            let solitude = if character.keeps_apart() {
                0.6 + apart(centre) / 6.0
            } else {
                1.0
            };
            let mine = piece_favourite(uid);
            if piece.has(Use::Sit { seats: 1 }) {
                if free {
                    options.push((Act::Sit(uid), 0.9 * solitude * mine));
                    options.push((Act::Nap(uid), if lazy { 1.3 } else { 0.7 } * mine));
                } else if kind == TemperamentKind::Grump
                    && mine > 1.0
                    && let Some(sitter) = self.on_piece(uid).filter(|sitter| *sitter != id)
                {
                    // Somebody is in its chair.
                    options.push((Act::GrumbleAt(sitter), 1.6));
                }
            }
            let guest = visiting && self.minds[index].staying;
            if piece.has(Use::Sleep) && free && guest && spare_bed(household, house, likings, uid) {
                // A friend staying over sleeps in the guest bedroll if there is one, or in a bed
                // nobody at home has made their own.
                let bedroll = if piece.id == "bedroll" { 2.0 } else { 1.0 };
                options.push((Act::Sleep(uid), 1.2 * bedroll));
                options.push((Act::CurlUp(uid), 0.6 * bedroll * solitude));
            }
            if piece.has(Use::Sleep) && free && !visiting {
                let basket = piece.id == "basket";
                let fits = if resident.is_little() == basket {
                    1.4
                } else {
                    0.8
                };
                options.push((Act::Sleep(uid), fits * if lazy { 1.5 } else { 1.0 } * mine));
                options.push((Act::CurlUp(uid), fits * 0.8 * solitude * mine));
                if (character.playful() && kind == TemperamentKind::Troublemaker)
                    || resident.is_little()
                {
                    options.push((Act::Bounce(uid), 0.7));
                }
            }
            if piece.flat {
                options.push((Act::Sprawl(uid), if lazy { 1.2 } else { 0.35 }));
            }
            if piece.family == Family::Tables && !visiting {
                options.push((Act::SitAt(uid), 0.5 * solitude));
                if resident.is_little() && !self.minds[index].drew {
                    let of = subject(household, id, &others);
                    options.push((Act::Draw(uid, of), 0.5 + character.axes.curiosity * 0.4));
                }
            }
            if piece.family == Family::Plants && !visiting {
                options.push((Act::Tend(uid), 0.25 + character.axes.affection * 0.3));
            }
            if piece.family == Family::Lights && character.fusses() {
                options.push((Act::SwitchLamp(uid), 0.25));
            }
            if piece.family == Family::Shelves && !shown_on(house, uid).is_empty() {
                let lots = if character.studies() || visiting {
                    1.0
                } else {
                    0.4
                };
                options.push((Act::Browse(uid), lots));
            }
            if piece.has(Use::Play) {
                options.push((Act::Play(uid), 1.0 * mine));
                for other in &others {
                    let bond = household.bond(id, *other);
                    if character.playful() && bond.friction < Band::High {
                        options.push((
                            Act::PlayWith(uid, *other),
                            0.6 + band(bond.playfulness) * 0.6,
                        ));
                    }
                }
                if character.shows_off() && !others.is_empty() {
                    options.push((Act::ShowOffToy(uid), if visitors_here { 1.5 } else { 0.9 }));
                }
            }
            if piece.has(Use::Snack) {
                let loves_food = resident
                    .traveler
                    .character
                    .trait_ids
                    .contains(&Trait::FoodMotivated)
                    || character.has_habit(Habit::LooksFoodOver);
                options.push((Act::Snack(uid), if loves_food { 1.5 } else { 0.6 }));
            }
        }
        let seen = self.minds[index].seen.clone();
        for shown in &house.shown {
            let item = &shown.item;
            let Some(thing) = snapshot.item(item) else {
                continue;
            };
            if self.worn.iter().any(|(worn, _)| worn == item) {
                continue;
            }
            let precious = is_precious(thing);
            let souvenir = matches!(thing.source, DisplaySource::HillSouvenir { .. });
            let mine = if favourite(Liked::Shown { item: item.clone() }) {
                1.6
            } else {
                1.0
            };
            let mut inspect = if character.studies() && precious {
                1.7
            } else {
                1.0
            };
            // A visitor takes a good look at everything on show it has not seen yet, and most of
            // all at whatever stands out.
            if visiting && !seen.contains(item) {
                inspect *= if precious || souvenir { 2.4 } else { 1.8 };
            }
            // Something new since the room was arranged draws the inquisitive first.
            if self.novel.contains(item)
                && (character.studies() || kind == TemperamentKind::Oddball)
            {
                inspect *= 2.2;
            }
            options.push((Act::Inspect(item.clone()), inspect * mine));
            // Something found by a close friend, or by itself, is something to remember by.
            let found_by_friend = thing.finder_name.as_deref().is_some_and(|finder| {
                finder == resident.name
                    || household
                        .everyone()
                        .any(|other| other.name == finder && household.bond(id, other.id).close())
            });
            options.push((
                Act::Remember(item.clone()),
                if found_by_friend { 1.2 } else { 0.45 } * mine,
            ));
            if character.shows_off() && !visiting {
                for other in &others {
                    // A show-off's best audience is a visitor, and its best find the oddest.
                    let audience = if household.is_visitor(*other) {
                        2.2
                    } else {
                        1.0
                    };
                    let prize = if precious {
                        1.6
                    } else if souvenir {
                        1.4
                    } else {
                        1.1
                    };
                    options.push((Act::ShowTo(item.clone(), *other), audience * prize * mine));
                }
            }
            if character.fusses() {
                options.push((Act::FussWith(item.clone()), 0.7));
            }
            if toy_like(thing) {
                options.push((Act::PlayWithFind(item.clone()), 0.9 * mine));
            }
            if !visiting && wearable(thing).is_some() {
                let dressy = if character.shows_off()
                    || matches!(
                        kind,
                        TemperamentKind::Oddball | TemperamentKind::Troublemaker
                    ) {
                    0.7
                } else {
                    0.2
                };
                options.push((Act::TryOn(item.clone()), dressy));
            }
        }
        for other in &others {
            let bond = household.bond(id, *other);
            let family = household.family(id, *other);
            let seek = if bond.close() {
                1.6
            } else if family {
                1.3
            } else {
                1.0
            };
            let shy = if bond.friction >= Band::High {
                0.3
            } else {
                1.0
            };
            // A visitor came to see its friends.
            let company = if visiting || household.is_visitor(*other) {
                1.4
            } else {
                1.0
            };
            let weight = seek * shy * company;
            options.push((Act::Greet(*other), 0.5 * weight));
            if character.hugs() && (bond.warmth >= Band::Medium || family) {
                options.push((Act::Hug(*other), 0.9 * weight));
            }
            if character.playful() {
                options.push((
                    Act::PlayTogether(*other),
                    (0.5 + band(bond.playfulness) * 0.6) * weight,
                ));
            }
            if character.teases() {
                options.push((Act::Tease(*other), 0.8));
            }
            let little = household.resident(*other).is_some_and(|r| r.is_little());
            let guards = kind == TemperamentKind::Guardian || household.family(id, *other);
            if little && !resident.is_little() && guards {
                options.push((Act::Comfort(*other), 1.0 * weight));
            }
            if !character.keeps_apart() && seats.len() >= 2 {
                options.push((Act::SitTogether(*other), 0.8 * weight));
            }
        }
        let floor = Floor::of(house);
        let rooms_now: Vec<(Id, Option<u8>)> = self
            .minds
            .iter()
            .zip(&self.actors)
            .map(|(mind, actor)| (mind.id, house.room_of_point(actor.pos)))
            .collect();
        let mind = &mut self.minds[index];
        // Which room to be in: the one it is in, until that palls, and then, more often than
        // not, one of the others, however little there is to do there.
        let here = house.room_of_point(me);
        let elsewhere: Vec<u8> = (0..house.rooms.len() as u8)
            .filter(|room| Some(*room) != here)
            .collect();
        let leaving = !elsewhere.is_empty() && mind.dice.next() < mind.restless * LEAVE_ROOM;
        let room = if leaving {
            let pick = (mind.dice.next() * elsewhere.len() as f32) as usize;
            elsewhere[pick.min(elsewhere.len() - 1)]
        } else {
            here.unwrap_or(0)
        };
        // A look round somewhere in it.
        let area = &house.rooms[usize::from(room)];
        let wander = (
            i32::from(area.x) + (mind.dice.next() * f32::from(area.width)) as i32,
            i32::from(area.y) + (mind.dice.next() * f32::from(area.depth)) as i32,
        );
        if let Some(tile) = floor.nearest_open_in(wander, room) {
            let far = ((tile.0 as f32 + 0.5 - me.0).powi(2) + (tile.1 as f32 + 0.5 - me.1).powi(2))
                .sqrt();
            if far > 1.5 {
                let weight = if leaving { 1.2 } else { 0.5 };
                options.push((Act::Wander(tile.0 as u8, tile.1 as u8), weight));
            }
        }
        // Only what is in that room comes to mind, and the others, wherever they are.
        let in_room = |act: &Act| {
            let there = match (act.piece(), act.item()) {
                (Some(uid), _) => house
                    .piece(uid)
                    .and_then(|placed| house.room_of_point(room::footprint(placed).centre())),
                (_, Some(item)) => item_room(house, item),
                (None, None) => return true,
            };
            there == Some(room)
        };
        if options.iter().any(|(act, _)| in_room(act)) {
            options.retain(|(act, _)| in_room(act));
        }
        // Someone in another room can wait a while, for one only just come into this one.
        let settling = 0.35 + mind.restless * 0.65;
        for (act, weight) in &mut options {
            if let Some(other) = act.partner()
                && rooms_now
                    .iter()
                    .find(|(id, _)| *id == other)
                    .and_then(|(_, at)| *at)
                    != Some(room)
            {
                *weight *= settling;
            }
        }
        let whimsy = character.whimsy();
        let mut best: Option<(Act, f32)> = None;
        for (act, weight) in options {
            let drive = mind.drives[act.drive().index()];
            let again = if mind.last.as_ref() == Some(&act) {
                0.3
            } else {
                1.0
            };
            let score =
                (0.15 + drive) * weight * again * (1.0 - whimsy + mind.dice.next() * whimsy * 2.0);
            if best.as_ref().is_none_or(|(_, top)| score > *top) {
                best = Some((act, score));
            }
        }
        best.map(|(act, _)| act)
    }

    /// Who is asleep: for tests.
    #[cfg(test)]
    pub fn asleep(&self, id: Id) -> bool {
        self.actor(id)
            .is_some_and(|actor| actor.pose.asleep() && !actor.walking())
    }

    /// The act someone is about, if it is about one, and whether it was asked for.
    #[cfg(test)]
    pub fn current(&self, id: Id) -> Option<(Act, bool)> {
        let mind = &self.minds[self.index(id)?];
        mind.plan
            .as_ref()
            .map(|plan| (plan.act.clone(), plan.asked))
    }
}

impl Mind {
    fn new(id: Id, presence: Presence, dawdle_until: f32, mut dice: Dice) -> Self {
        let drives = [
            dice.between(0.15, 0.5),
            dice.between(0.2, 0.6),
            dice.between(0.2, 0.6),
            dice.between(0.3, 0.7),
            dice.between(0.1, 0.4),
        ];
        Self {
            id,
            presence,
            drives,
            queue: VecDeque::new(),
            plan: None,
            dawdle_until,
            last: None,
            dice,
            handled: None,
            seen: Vec::new(),
            room: None,
            entered: 0.0,
            restless: 0.0,
            drew: false,
            staying: false,
        }
    }
}

/// The way in: a point in the front door's doorway, and the open tile inside it. A house with no
/// front door is come into across the first room's near edge.
fn door(house: &House, floor: &Floor) -> ((f32, f32), (i32, i32)) {
    if let Some(wall) = house.front_door() {
        let (x, y) = (i32::from(wall.x), i32::from(wall.y));
        let inside = floor.nearest_open_in((x, y), wall.room).unwrap_or((x, y));
        let (mx, my) = wall.middle();
        let outside = match wall.side {
            formiga_home_contract::WallSide::North => (mx, my + 0.15),
            formiga_home_contract::WallSide::West => (mx + 0.15, my),
        };
        return (outside, inside);
    }
    let first = &house.rooms[0];
    let wanted = (
        i32::from(first.x + first.width / 2),
        i32::from(first.y + first.depth) - 1,
    );
    let inside = floor.nearest_open_in(wanted, 0).unwrap_or(wanted);
    let outside = (
        inside.0 as f32 + 0.5,
        f32::from(first.y + first.depth) + 0.9,
    );
    (outside, inside)
}

/// Whether a bed is free for a guest: nobody at home has made it their favourite.
fn spare_bed(household: &Household, house: &House, likings: &[Liking], uid: u16) -> bool {
    !household.residents.iter().any(|resident| {
        favourites(likings, house, resident.id)
            .iter()
            .any(|(kind, liked)| *kind == Kind::Bed && house.of_liked(liked) == Some(uid))
    })
}

/// Whom a little one draws: its own adult if it is home, or else whoever in the house it is
/// warmest towards, or else itself.
fn subject(household: &Household, little: Id, present: &[Id]) -> Id {
    let parent = household.resident(little).and_then(|r| r.parent());
    if let Some(parent) = parent.filter(|parent| present.contains(parent)) {
        return parent;
    }
    present
        .iter()
        .copied()
        .max_by_key(|other| {
            let bond = household.bond(little, *other);
            (bond.warmth, bond.familiarity, std::cmp::Reverse(*other))
        })
        .unwrap_or(little)
}

fn band(band: Band) -> f32 {
    match band {
        Band::None => 0.0,
        Band::Low => 0.33,
        Band::Medium => 0.66,
        Band::High => 1.0,
    }
}

/// Rare or delicate: something that would rather be kept in a case.
fn is_precious(item: &DisplayItem) -> bool {
    item.modes.first() == Some(&DisplayMode::Case)
}

/// The finds that are toys, which a resident can play with as well as look at.
fn toy_like(item: &DisplayItem) -> bool {
    const TOYS: [u8; 15] = [
        13, 18, 25, 32, 44, 45, 46, 53, 78, 85, 126, 129, 132, 151, 159,
    ];
    match &item.source {
        DisplaySource::DesktopFind { variant } => TOYS.contains(variant),
        DisplaySource::HillSouvenir { id } => id == "chest_marble",
        DisplaySource::HomeMemento { memento } => *memento == MementoKind::Pebble,
        DisplaySource::Unknown => false,
    }
}

/// How a find looks pinned on, in its colony's inks, if it is small enough to wear: a find from
/// the scrapbook that does not have to stand on the floor. Worn for a while in Home, it changes
/// nothing on Desktop.
fn wearable(item: &DisplayItem) -> Option<AccessoryArt> {
    let DisplaySource::DesktopFind { variant } = item.source else {
        return None;
    };
    let ink = item.ink?;
    (item.modes.first() != Some(&DisplayMode::Floor)).then(|| AccessoryArt {
        accessory: Accessory::Pin(variant),
        ink: ink.to_art(),
    })
}

/// What is shown on a piece.
fn shown_on(house: &House, uid: u16) -> Vec<DisplayId> {
    house
        .shown
        .iter()
        .filter(|shown| matches!(shown.at, At::On { piece, .. } if piece == uid))
        .map(|shown| shown.item.clone())
        .collect()
}

/// Every place on a piece someone can sit or lie: along its width for a seat, head and foot for
/// a bed.
pub fn seat_points(
    placed: &formiga_home_contract::PlacedPiece,
    piece: &catalog::Piece,
) -> Vec<(f32, f32)> {
    let (w, d) = (f32::from(piece.size.0), f32::from(piece.size.1));
    let points: Vec<(f32, f32)> = if piece.has(Use::Sit { seats: 1 }) {
        let seats = piece.seats().max(1);
        (0..seats)
            .map(|seat| ((f32::from(seat) + 0.5) * w / f32::from(seats), d * 0.6))
            .collect()
    } else if piece.has(Use::Sleep) && piece.size.1 >= 2 {
        vec![(w / 2.0, 0.75), (w / 2.0, d - 0.55)]
    } else if piece.has(Use::Sleep) {
        vec![(w / 2.0, d / 2.0)]
    } else {
        Vec::new()
    };
    points
        .into_iter()
        .map(|point| {
            let (x, y) = catalog::turned(point, piece.size, placed.turn);
            (f32::from(placed.x) + x, f32::from(placed.y) + y)
        })
        .collect()
}

/// Tiles to stand on, nearest first, and the point to look at.
type Viewpoint = (Vec<(i32, i32)>, (f32, f32));

/// Where someone stands to look at something shown, and the point they look at.
fn item_spot(house: &House, floor: &Floor, item: &DisplayId) -> Option<Viewpoint> {
    let shown = house.shown.iter().find(|shown| &shown.item == item)?;
    match shown.at {
        At::On { piece, slot } => {
            let placed = house.piece(piece)?;
            let footprint = room::footprint(placed);
            let at = room::surfaces(placed).get(usize::from(slot))?.at;
            Some((
                floor.beside(
                    placed.x,
                    placed.y,
                    footprint.w,
                    footprint.d,
                    catalog::front(placed.turn),
                ),
                at,
            ))
        }
        At::Wall { room, side, at } => {
            let wall = house.wall(room, side, at)?;
            let (tile, look) = ((i32::from(wall.x), i32::from(wall.y)), wall.middle());
            let near = floor.nearest_open_in(tile, room)?;
            let mut around = vec![near];
            around.extend(
                floor
                    .beside(near.0 as u8, near.1 as u8, 1, 1, (0, 1))
                    .into_iter()
                    .take(2),
            );
            Some((around, look))
        }
        At::Floor { x, y } => Some((
            floor.beside(x, y, 1, 1, (0, 1)),
            (f32::from(x) + 0.5, f32::from(y) + 0.5),
        )),
    }
}

/// The room something shown is in.
fn item_room(house: &House, item: &DisplayId) -> Option<u8> {
    let shown = house.shown.iter().find(|shown| &shown.item == item)?;
    match shown.at {
        At::On { piece, .. } => house
            .piece(piece)
            .and_then(|placed| house.room_at(i32::from(placed.x), i32::from(placed.y))),
        At::Wall { room, .. } => Some(room),
        At::Floor { x, y } => house.room_at(i32::from(x), i32::from(y)),
    }
}

/// Whether the house still has what an act is about.
fn still_there(act: &Act, house: &House, snapshot: &HomeSnapshot) -> bool {
    if let Some(uid) = act.piece()
        && house.piece(uid).is_none()
    {
        return false;
    }
    if let Some(item) = act.item() {
        return snapshot.item(item).is_some()
            && house.shown.iter().any(|shown| &shown.item == item);
    }
    true
}

/// What someone can be asked to do with something in the room, for the owner's menu. `present`
/// is everyone in the house just now, and `guests` whoever of them is staying over.
pub fn choices(
    household: &Household,
    house: &House,
    snapshot: &HomeSnapshot,
    present: &[Id],
    guests: &[Id],
    id: Id,
    target: &crate::scene::Target,
) -> Vec<Act> {
    use crate::scene::Target;
    let Some(resident) = household.resident(id) else {
        return Vec::new();
    };
    let character = &resident.character;
    let visiting = household.is_visitor(id);
    let others: Vec<Id> = present
        .iter()
        .copied()
        .filter(|other| *other != id)
        .collect();
    let mut acts = Vec::new();
    match target {
        Target::Floor(x, y) => acts.push(Act::GoTo(*x, *y)),
        Target::Piece(uid) => {
            let Some(placed) = house.piece(*uid) else {
                return acts;
            };
            let Some(piece) = catalog::piece(&placed.piece) else {
                return acts;
            };
            if piece.has(Use::Sit { seats: 1 }) {
                acts.extend([Act::Sit(*uid), Act::Relax(*uid), Act::Nap(*uid)]);
            }
            if piece.has(Use::Sleep) && !visiting {
                acts.extend([Act::Sleep(*uid), Act::CurlUp(*uid), Act::Bounce(*uid)]);
                if !resident.is_little() && piece.size.1 >= 2 {
                    for other in &others {
                        if household
                            .resident(*other)
                            .is_some_and(|r| r.parent() == Some(id))
                        {
                            acts.push(Act::InviteLittle(*uid, *other));
                        }
                    }
                }
            } else if piece.has(Use::Sleep) && guests.contains(&id) {
                // A friend staying over may be shown to a bed.
                acts.extend([Act::Sleep(*uid), Act::CurlUp(*uid)]);
            }
            if piece.family == Family::Tables {
                acts.push(Act::SitAt(*uid));
                if resident.is_little() && !visiting {
                    acts.push(Act::Draw(*uid, subject(household, id, &others)));
                }
            }
            if piece.family == Family::Lights {
                acts.push(Act::SwitchLamp(*uid));
            }
            if piece.family == Family::Plants {
                acts.push(Act::Tend(*uid));
            }
            if piece.family == Family::Shelves {
                acts.push(Act::Browse(*uid));
            }
            if piece.flat {
                acts.push(Act::Sprawl(*uid));
            }
            if piece.has(Use::Play) {
                acts.push(Act::Play(*uid));
                acts.extend(others.iter().map(|other| Act::PlayWith(*uid, *other)));
                acts.push(Act::ShowOffToy(*uid));
            }
            if piece.has(Use::Snack) {
                acts.push(Act::Snack(*uid));
                acts.extend(others.iter().map(|other| Act::Share(*uid, *other)));
            }
            if acts.is_empty() || piece.flat {
                let footprint = room::footprint(placed);
                let (cx, cy) = footprint.centre();
                let floor = Floor::of(house);
                let beside = floor
                    .beside(
                        placed.x,
                        placed.y,
                        footprint.w,
                        footprint.d,
                        catalog::front(placed.turn),
                    )
                    .first()
                    .copied();
                if piece.flat {
                    acts.insert(0, Act::GoTo(cx as u8, cy as u8));
                } else if let Some((x, y)) = beside {
                    acts.push(Act::GoTo(x as u8, y as u8));
                }
            }
        }
        Target::Shown(item) => {
            acts.push(Act::Inspect(item.clone()));
            acts.push(Act::Remember(item.clone()));
            acts.extend(others.iter().map(|other| Act::ShowTo(item.clone(), *other)));
            let thing = snapshot.item(item);
            if thing.is_some_and(toy_like) {
                acts.push(Act::PlayWithFind(item.clone()));
            }
            if !visiting && thing.and_then(wearable).is_some() {
                acts.push(Act::TryOn(item.clone()));
            }
            if character.fusses() {
                acts.push(Act::FussWith(item.clone()));
            }
        }
        Target::Resident(other) if *other != id => {
            let other = *other;
            let bond = household.bond(id, other);
            let family = household.family(id, other);
            acts.push(Act::Greet(other));
            acts.push(Act::SitTogether(other));
            if character.playful() {
                acts.push(Act::PlayTogether(other));
            }
            if character.teases() {
                acts.push(Act::Tease(other));
            }
            let little = household.resident(other).is_some_and(|r| r.is_little());
            if character.hugs() || family || little {
                acts.push(Act::Comfort(other));
            }
            if character.hugs() && (bond.warmth >= Band::Medium || family) {
                acts.push(Act::Hug(other));
            }
        }
        Target::Resident(_) => {}
    }
    acts
}

#[cfg(test)]
mod tests;
