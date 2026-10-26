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
use crate::body::Body;
use crate::catalog::{self, Use};
use crate::character::Drive;
use crate::house::{At, House};
use crate::household::{Household, Id};
use crate::keepsakes;
use crate::path::Floor;
use crate::placement;
use crate::projects::Count;
use formiga_art::{AccessoryArt, ExpressionKind};
use formiga_core::{Accessory, ActionKind, Gesture, Habit, TemperamentKind};
use formiga_home_contract::limits::MAX_TOGETHER;
use formiga_home_contract::{
    DisplayId, DisplayItem, DisplayMode, DisplaySource, HomeSnapshot, Liked, Liking, MementoKind,
    Together, TravelerId,
};
use formiga_travel::Band;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

mod act;
mod choices;
mod poses;
mod visitors;
mod whims;

pub use act::Act;
pub use choices::{choices, seat_points};
use choices::{item_spot, still_there};
use poses::{joining_pose, leading_pose};

/// How many things the owner can ask of one resident at once, the one under way included.
pub const QUEUE_LIMIT: usize = 3;

/// How long in one room before another calls, in seconds: for a homebody, and for one who gets
/// about the house most.
const ROOM_RESTLESS: (f32, f32) = (220.0, 70.0);

/// How many times something must be chosen before it is a favourite.
pub const FAVOURITE_AFTER: u16 = 3;

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
    /// Someone has done something the household's projects count.
    Did(Id, Count),
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
    /// Sent out to the desktop by the owner, and on the way to the door.
    SteppingOut,
    /// Out on the desktop, until the owner has them in again.
    Out,
}

impl Presence {
    fn in_house(self) -> bool {
        matches!(self, Self::Home | Self::Visiting { .. })
    }

    /// Somewhere in the house where they can be seen, if only on the way out of it.
    fn seen(self) -> bool {
        self.in_house() || matches!(self, Self::Leaving | Self::SteppingOut)
    }
}

/// Where someone is, as the household's cells show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inside {
    /// In the house.
    Here,
    /// A friend who has not come over yet.
    Coming,
    /// A friend who has gone home again.
    GoneHome,
    /// Sent out to the desktop.
    Out,
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
    /// How dark it is at home.
    dark: f32,
    /// The house as it was last lived in, and its open floor: found again only when the house
    /// changes, not every step.
    floor: Option<(House, Rc<Floor>)>,
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
            dark: 0.0,
            floor: None,
        }
    }

    /// The open floor of `house`, as last found if the house is as it was.
    fn floor(&mut self, house: &House) -> Rc<Floor> {
        if let Some((of, floor)) = &self.floor
            && of == house
        {
            return Rc::clone(floor);
        }
        let floor = Rc::new(Floor::of(house));
        self.floor = Some((house.clone(), Rc::clone(&floor)));
        floor
    }

    fn index(&self, id: Id) -> Option<usize> {
        self.minds.iter().position(|mind| mind.id == id)
    }

    #[cfg(test)]
    pub fn actor(&self, id: Id) -> Option<&Actor> {
        self.actors.iter().find(|actor| actor.id == id)
    }

    /// How dark it is at home, from nothing by day to about a half at night: the later it is,
    /// the sooner everyone tires and the longer they sleep.
    pub fn set_dark(&mut self, dark: f32) {
        self.dark = dark.clamp(0.0, 1.0);
    }

    /// What has happened since the window last asked.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// For tests: as if this had just happened.
    #[cfg(test)]
    pub fn push_event(&mut self, event: Event) {
        self.events.push(event);
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

    /// Everyone who can be seen in the house, on the way out of it included: whom Desktop keeps
    /// indoors. Everyone else is out on the desktop.
    pub fn indoors(&self) -> Vec<Id> {
        self.minds
            .iter()
            .filter(|mind| mind.presence.seen())
            .map(|mind| mind.id)
            .collect()
    }

    /// Where someone is, for the household's cells.
    pub fn inside(&self, id: Id) -> Option<Inside> {
        Some(match self.minds[self.index(id)?].presence {
            Presence::Home | Presence::Visiting { .. } => Inside::Here,
            Presence::Expected { .. } => Inside::Coming,
            Presence::Leaving | Presence::Gone => Inside::GoneHome,
            Presence::SteppingOut | Presence::Out => Inside::Out,
        })
    }

    /// Residents the owner keeps out on the desktop, from the moment the house opens.
    pub fn start_out(&mut self, ids: &[Id]) {
        for index in 0..self.minds.len() {
            if self.minds[index].presence == Presence::Home && ids.contains(&self.minds[index].id) {
                self.minds[index].presence = Presence::Out;
                self.actors[index].hidden = true;
            }
        }
    }

    /// The owner sends someone in the house out to the desktop: whatever it was doing it leaves,
    /// and out it goes by the front door. A friend still to come stays where it is. Whether
    /// anything changed.
    pub fn send_out(&mut self, house: &House, id: Id, now: f32) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        match self.minds[index].presence {
            Presence::Home | Presence::Visiting { .. } => {}
            Presence::Expected { .. } => {
                self.minds[index].presence = Presence::Out;
                return true;
            }
            _ => return false,
        }
        self.drop_everything(index, now);
        self.minds[index].handled = None;
        self.minds[index].queue.clear();
        self.minds[index].staying = false;
        let floor = self.floor(house);
        let (outside, inside) = door(house, &floor);
        let actor = &mut self.actors[index];
        actor.get_down();
        actor.lift = 0.0;
        let mut route = way(&floor, actor, inside);
        route.push(outside);
        actor.walk(route);
        self.minds[index].presence = Presence::SteppingOut;
        true
    }

    /// The owner has someone in from the desktop: in it comes by the front door, or turns back
    /// if it was on its way out. A friend comes in as a visitor would. Whether anything changed.
    pub fn bring_in(&mut self, household: &Household, house: &House, id: Id, now: f32) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        let floor = self.floor(house);
        match self.minds[index].presence {
            Presence::Out | Presence::Gone | Presence::Expected { .. } => {
                self.knock(household, house, &floor, index, now);
                true
            }
            Presence::SteppingOut | Presence::Leaving => {
                let (_, inside) = door(house, &floor);
                let stay = self.minds[index].dice.between(150.0, 210.0);
                self.minds[index].presence = if household.is_visitor(id) {
                    Presence::Visiting { until: now + stay }
                } else {
                    Presence::Home
                };
                self.minds[index].dawdle_until = now + 1.0;
                let actor = &mut self.actors[index];
                let route = way(&floor, actor, inside);
                actor.walk(route);
                true
            }
            Presence::Home | Presence::Visiting { .. } => false,
        }
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
        // Patted where it is, up on a piece or not, and stopped for it: whatever it was walking
        // to can wait, and the pat is answered rather than walked through.
        let actor = &mut self.actors[index];
        actor.stop();
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
        actor.stop();
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

    /// Set down where it was let go, or, over furniture, on the open tile nearest it.
    pub fn put_down(&mut self, house: &House, id: Id, now: f32) {
        let Some(index) = self.index(id) else { return };
        let floor = self.floor(house);
        let at = Floor::tile_of(self.actors[index].pos);
        let actor = &mut self.actors[index];
        if !floor.open(at.0, at.1) {
            let tile = floor.nearest_open(at).unwrap_or((0, 0));
            actor.pos = (tile.0 as f32 + 0.5, tile.1 as f32 + 0.5);
        }
        actor.stop();
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
        let floor = self.floor(house);
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
            (Presence::SteppingOut, ..) => format!("{name} is off out to the desktop."),
            (Presence::Out, ..) => format!("{name} is out on the desktop."),
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
        let floor = self.floor(house);
        for index in 0..self.minds.len() {
            let Some(resident) = household.resident(self.minds[index].id) else {
                continue;
            };
            match self.minds[index].presence {
                Presence::Expected { at } if now >= at => {
                    self.knock(household, house, &floor, index, now);
                }
                Presence::Expected { .. } | Presence::Gone | Presence::Out => continue,
                Presence::SteppingOut => {
                    self.actors[index].advance(dt);
                    if !self.actors[index].walking() {
                        self.actors[index].hidden = true;
                        self.minds[index].presence = Presence::Out;
                    }
                    continue;
                }
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
                    // Late at night, going home is further than it was.
                    let late = if self.dark >= 0.45 { 2.0 } else { 1.0 };
                    let asks = resident.character.asks_to_stay(warmth) * late;
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
            let mut rates = resident.character.drive_rates();
            // As the light goes, everyone tires, the little ones soonest.
            let tiring = if resident.is_little() { 3.5 } else { 2.2 };
            rates[Drive::Rest.index()] *= 1.0 + self.dark * tiring;
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
            // Nobody stands about on the furniture, or stays up on a piece once done with it: the
            // open floor is a step away, and it gets down and walks there.
            let seated = self.minds[index]
                .plan
                .as_ref()
                .is_some_and(|plan| plan.seat.is_some());
            let actor = &mut self.actors[index];
            let tile = Floor::tile_of(actor.pos);
            let inside = house.room_of_point(actor.pos).is_some();
            if !seated
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
                // Told to stop waiting while still on its way, it gets there first.
                if self.actors[index].walking() {
                    return;
                }
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
        let body = self.actors[index].body();
        if !still_there(&act, house, snapshot)
            || self.worn_by_another(&act, id)
            || !suits(body, house, &act)
        {
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
        let mut mine = seats_for(body, house, &seats);
        // Whoever it is done with joins in, unless they are asleep for being asked or handled.
        let joining = partner.filter(|&o| {
            self.minds[o].handled.is_none()
                && !self.minds[o]
                    .plan
                    .as_ref()
                    .is_some_and(|p| p.asked && p.part == Part::Leads)
        });
        // Sitting or lying down together wants somewhere for each body: a seat is taken only if
        // whoever joins has another they can get up onto and fit, in the same bed to turn in
        // together. With none, it is not done.
        if let Some(o) = joining {
            let theirs = seats_for(self.actors[o].body(), house, &seats);
            match &act {
                Act::SitTogether(_) => mine.retain(|seat| theirs.iter().any(|other| other != seat)),
                Act::InviteLittle(uid, _) => mine.retain(|seat| {
                    theirs
                        .iter()
                        .any(|other| other != seat && other.piece == *uid)
                }),
                _ => {}
            }
        }
        let spot = self.spot_for(house, floor, index, &act, &mine);
        let Some((tile, seat, face)) = spot else {
            self.minds[index].dawdle_until = now + 1.0;
            return;
        };
        let actor = &mut self.actors[index];
        let route = way(floor, actor, tile);
        match seat {
            Some(seat) => actor.walk_onto(route, seat.piece, seat.at, seat.lift, seat.facing_right),
            None => actor.walk(route),
        }
        self.minds[index].plan = Some(Plan {
            act: act.clone(),
            part: Part::Leads,
            asked,
            stage: Stage::Going,
            seat,
            face,
        });
        if let Some(o) = joining {
            self.end(o, now, false);
            let mut free = seats_for(self.actors[o].body(), house, &seats);
            if let Some(mine) = seat {
                free.retain(|other| *other != mine);
            }
            let (their_tile, their_seat) = Self::join_spot(house, floor, &act, tile, seat, &free);
            let other = &mut self.actors[o];
            let route = way(floor, other, their_tile);
            match their_seat {
                Some(seat) => {
                    other.walk_onto(route, seat.piece, seat.at, seat.lift, seat.facing_right);
                }
                None => other.walk(route),
            }
            self.minds[o].plan = Some(Plan {
                act,
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
        // Anyone stopped up on a piece with no seat to sit for, for a pat say, keeps the place
        // nearest it until it walks down.
        for (mind, actor) in self.minds.iter().zip(&self.actors) {
            let Some(uid) = actor.on_piece.filter(|_| !actor.walking()) else {
                continue;
            };
            if mind.plan.as_ref().is_some_and(|plan| plan.seat.is_some()) {
                continue;
            }
            let distance =
                |seat: &Seat| (seat.at.0 - actor.pos.0).powi(2) + (seat.at.1 - actor.pos.1).powi(2);
            if let Some(nearest) = seats
                .iter()
                .enumerate()
                .filter(|(_, seat)| seat.piece == uid)
                .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
                .map(|(index, _)| index)
            {
                seats.remove(nearest);
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
            let footprint = placement::footprint(placed);
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
            let footprint = placement::footprint(placed);
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
                let footprint = placement::footprint(placed);
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
        house: &House,
        floor: &Floor,
        act: &Act,
        leader: (i32, i32),
        leader_seat: Option<Seat>,
        seats: &[Seat],
    ) -> ((i32, i32), Option<Seat>) {
        let settles = matches!(act, Act::SitTogether(_) | Act::InviteLittle(..));
        if settles {
            let near = |uid: u16| {
                let placed = house.piece(uid)?;
                let footprint = placement::footprint(placed);
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
                return (near(seat.piece).unwrap_or(leader), Some(seat));
            }
        }
        let around = floor.beside(leader.0 as u8, leader.1 as u8, 1, 1, (1, 0));
        (*around.first().unwrap_or(&leader), None)
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
                // Still on the way, it walks the rest and climbs up when it gets there.
                if !actor.walking() {
                    actor.settle_on(seat.piece, seat.at, seat.lift, seat.facing_right);
                }
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
        match &plan.act {
            Act::SwitchLamp(uid) if plan.part == Part::Leads => self.switch_lamp(*uid),
            Act::TryOn(item) if plan.part == Part::Leads => self.put_on(snapshot, index, item),
            act => self.take_in(house, index, act),
        }
        let (pose, seconds) = match plan.part {
            Part::Joins(leader) => (
                joining_pose(household, resident, leader, &plan.act),
                f32::MAX,
            ),
            Part::Leads => {
                let dark = self.dark;
                leading_pose(
                    snapshot,
                    resident,
                    &plan.act,
                    dark,
                    &mut self.minds[index].dice,
                )
            }
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

    /// A lamp that was on goes off, and one that was off comes on.
    fn switch_lamp(&mut self, uid: u16) {
        match self.lamps_off.iter().position(|off| *off == uid) {
            Some(at) => {
                self.lamps_off.remove(at);
            }
            None => self.lamps_off.push(uid),
        }
    }

    /// A find pinned on for a while, if it is small enough to wear.
    fn put_on(&mut self, snapshot: &HomeSnapshot, index: usize, item: &DisplayId) {
        if let Some(dress) = snapshot.item(item).and_then(wearable) {
            self.worn.push((item.clone(), self.minds[index].id));
            self.actors[index].wear(Some(dress));
        }
    }

    /// Whatever an act is about has been had a good look at, everything on a shelf looked over
    /// with it, and none of it is new any more.
    fn take_in(&mut self, house: &House, index: usize, act: &Act) {
        let seen = &mut self.minds[index].seen;
        if let Some(item) = act.item()
            && !seen.contains(item)
        {
            seen.push(item.clone());
        }
        if let Act::Browse(uid) = act {
            for item in shown_on(house, *uid) {
                if !seen.contains(&item) {
                    seen.push(item);
                }
            }
        }
        if let Some(item) = act.item() {
            self.novel.retain(|novel| novel != item);
        }
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
                && let Some(count) = plan.act.counts()
            {
                self.events.push(Event::Did(id, count));
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
                    (Act::SwitchLamp(_) | Act::Browse(_) | Act::Wind(_), _, _) => None,
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
        // Done with a piece, it gets down by walking off it, as the next tick finds it still up
        // there with nothing to sit for.
        let actor = &mut self.actors[index];
        actor.stop();
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

/// The way from where someone is to `tile`. Up on a piece, it first gets down to the open floor
/// nearest, so nobody walks off through the rest of the piece.
fn way(floor: &Floor, actor: &Actor, tile: (i32, i32)) -> Vec<(f32, f32)> {
    let centre = |(x, y): (i32, i32)| (x as f32 + 0.5, y as f32 + 0.5);
    let mut route = floor
        .route(actor.pos, tile)
        .unwrap_or_else(|| vec![centre(tile)]);
    if actor.on_piece.is_some()
        && let Some(start) = floor.nearest_open(Floor::tile_of(actor.pos)).map(centre)
        && route.first() != Some(&start)
    {
        route.insert(0, start);
    }
    route
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

/// A bond's band as a number, from nothing for strangers to 1 for the closest.
pub fn band(band: Band) -> f32 {
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
        // A cup's trophy or medal is looked at, not played with.
        DisplaySource::BattleKeepsake { .. } | DisplaySource::Unknown => false,
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

/// Whether a body can do `act`: get up onto the piece it settles on, bounce on it, or stretch out
/// on a rug. Anything else, anybody can do.
pub fn suits(body: Body, house: &House, act: &Act) -> bool {
    match act {
        Act::Bounce(uid) => body.bounces() && body.settles_on_placed(house, *uid),
        Act::Sprawl(_) => body.sprawls(),
        _ if act.settles() => act
            .piece()
            .is_some_and(|uid| body.settles_on_placed(house, uid)),
        _ => true,
    }
}

/// Whether anything in the house has a seat a body can get up onto and fits.
fn somewhere_to_sit(body: Body, house: &House) -> bool {
    house.pieces.iter().any(|placed| {
        catalog::piece(&placed.piece)
            .is_some_and(|piece| body.settles_on(piece) && !seat_points(placed, piece).is_empty())
    })
}

/// The seats among `seats` a body can get up onto and fits.
fn seats_for(body: Body, house: &House, seats: &[Seat]) -> Vec<Seat> {
    seats
        .iter()
        .filter(|seat| body.settles_on_placed(house, seat.piece))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests;
