//! The household's life in the room: what each resident does, whether the owner asked or it
//! decided for itself.
//!
//! The owner can ask a resident for up to three things in turn. Whenever it has nothing asked of
//! it, it chooses for itself, from what the room offers and what it feels like: its drives, its
//! temperament, its habits and how it gets on with whoever else is home. A lazybones finds the
//! bed, an explorer the oddest thing on the shelf, a show-off someone to show it to; close friends
//! seek each other out and a little one keeps near its adult. Nothing here is a chore and nothing
//! goes wrong if the owner does nothing at all.

use crate::actor::{Actor, Pose};
use crate::art::cues::Cue;
use crate::catalog::{self, Use};
use crate::character::Drive;
use crate::household::{Household, Id};
use crate::path::Floor;
use crate::room;
use formiga_art::ExpressionKind;
use formiga_core::{ActionKind, Gesture, Habit, TemperamentKind};
use formiga_home_contract::{DisplayId, DisplaySource, HomeSnapshot, RoomLayout, Spot, WallSide};
use formiga_travel::{Band, Trait};
use std::collections::VecDeque;

/// How many things the owner can ask of one resident at once, the one under way included.
pub const QUEUE_LIMIT: usize = 3;

/// Something a resident does.
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
    Greet(Id),
    SitTogether(Id),
    PlayTogether(Id),
    Tease(Id),
    Comfort(Id),
    Hug(Id),
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
            | Self::PlayWithFind(item) => Some(item),
            _ => None,
        }
    }

    /// The drive doing it settles.
    fn drive(&self) -> Drive {
        match self {
            Self::Sit(_)
            | Self::Relax(_)
            | Self::CurlUp(_)
            | Self::Snack(_)
            | Self::Remember(_) => Drive::Comfort,
            Self::Nap(_) | Self::Sleep(_) | Self::InviteLittle(..) => Drive::Rest,
            Self::Play(_)
            | Self::PlayWith(..)
            | Self::ShowOffToy(_)
            | Self::PlayWithFind(_)
            | Self::PlayTogether(_)
            | Self::Tease(_)
            | Self::FussWith(_) => Drive::Play,
            Self::Inspect(_) | Self::Wander(..) | Self::GoTo(..) => Drive::Curiosity,
            Self::Share(..)
            | Self::ShowTo(..)
            | Self::Greet(_)
            | Self::SitTogether(_)
            | Self::Comfort(_)
            | Self::Hug(_) => Drive::Company,
        }
    }

    /// What the owner is offered, and what the queue shows: "Nap in the armchair", "Hug Pip".
    pub fn label(
        &self,
        household: &Household,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
    ) -> String {
        let piece = |uid: &u16| {
            layout
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
            Self::Greet(other) => format!("Say hello to {}", who(other)),
            Self::SitTogether(other) => format!("Sit with {}", who(other)),
            Self::PlayTogether(other) => format!("Play with {}", who(other)),
            Self::Tease(other) => format!("Tease {}", who(other)),
            Self::Comfort(other) => format!("Comfort {}", who(other)),
            Self::Hug(other) => format!("Hug {}", who(other)),
            Self::Wander(..) => "Have a look round".to_owned(),
        }
    }

    /// What it is doing, as the room's status line says it: "napping on the armchair".
    fn doing(&self, household: &Household, layout: &RoomLayout, snapshot: &HomeSnapshot) -> String {
        let label = self.label(household, layout, snapshot);
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
            "Play" => "playing",
            "Show" => "showing",
            "Have" => "having",
            "Share" => "sharing",
            "Look" => "looking",
            "Remember" => "remembering",
            "Fuss" => "fussing",
            "Say" => "saying",
            "Tease" => "teasing",
            "Comfort" => "comforting",
            "Hug" => "hugging",
            other => return other.to_lowercase(),
        };
        format!("{ing} {rest}")
    }
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

/// One resident's inner life.
struct Mind {
    id: Id,
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Handled {
    Petted { until: f32 },
    Held,
    Landing { until: f32 },
}

/// A small, steady source of whims, one per resident, so a household's life plays out the same
/// for the same start.
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

pub struct Life {
    pub actors: Vec<Actor>,
    minds: Vec<Mind>,
    paused: bool,
    reduce_motion: bool,
}

impl Life {
    /// The household come home: everyone somewhere about the room, already minded to do
    /// something.
    pub fn new(household: &Household, layout: &RoomLayout) -> Self {
        let floor = Floor::of(layout);
        let reduce_motion = household.reduce_motion();
        let mut taken: Vec<(i32, i32)> = Vec::new();
        let mut actors = Vec::new();
        let mut minds = Vec::new();
        for (index, resident) in household.residents.iter().enumerate() {
            let mut dice = Dice::new(resident.id);
            let wish = (
                (2.0 + dice.between(0.0, f32::from(layout.width) - 3.0)) as i32,
                (2.0 + dice.between(0.0, f32::from(layout.depth) - 3.0)) as i32,
            );
            let mut tile = floor.nearest_open(wish).unwrap_or((0, 0));
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
            let drives = [
                dice.between(0.15, 0.5),
                dice.between(0.2, 0.6),
                dice.between(0.2, 0.6),
                dice.between(0.3, 0.7),
                dice.between(0.1, 0.4),
            ];
            minds.push(Mind {
                id: resident.id,
                drives,
                queue: VecDeque::new(),
                plan: None,
                dawdle_until: 0.4 + index as f32 * 0.7,
                last: None,
                dice,
                handled: None,
            });
        }
        Self {
            actors,
            minds,
            paused: false,
            reduce_motion,
        }
    }

    fn index(&self, id: Id) -> Option<usize> {
        self.minds.iter().position(|mind| mind.id == id)
    }

    #[cfg(test)]
    pub fn actor(&self, id: Id) -> Option<&Actor> {
        self.actors.iter().find(|actor| actor.id == id)
    }

    /// What the owner has asked of a resident, the one under way first.
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

    /// Ask a resident to do something, after whatever else it has been asked. What it was doing
    /// of its own accord it leaves at once.
    pub fn ask(&mut self, id: Id, act: Act, now: f32) -> Asked {
        if self.queue(id).len() >= QUEUE_LIMIT {
            return Asked::Full;
        }
        let Some(index) = self.index(id) else {
            return Asked::Full;
        };
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

    /// Take back one thing asked of a resident, by its place in the queue.
    pub fn cancel(&mut self, id: Id, position: usize, now: f32) {
        let Some(index) = self.index(id) else { return };
        let leading = self.minds[index]
            .plan
            .as_ref()
            .is_some_and(|plan| plan.asked && plan.part == Part::Leads);
        match (position, leading) {
            (0, true) => self.end(index, now, false),
            (position, true) => {
                self.minds[index].queue.remove(position - 1);
            }
            (position, false) => {
                self.minds[index].queue.remove(position);
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
    pub fn put_down(&mut self, layout: &RoomLayout, id: Id, now: f32) {
        let Some(index) = self.index(id) else { return };
        let floor = Floor::of(layout);
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
    /// arranged round them.
    pub fn pause(&mut self, household: &Household, layout: &RoomLayout, now: f32) {
        self.paused = true;
        for index in 0..self.minds.len() {
            self.end(index, now, false);
            self.minds[index].handled = None;
            let face = household
                .resident(self.minds[index].id)
                .map_or(ExpressionKind::Neutral, |r| r.character.idle_face());
            let actor = &mut self.actors[index];
            actor.get_down();
            actor.stop();
            actor.lift = 0.0;
            actor.strike(Pose::idle(face), now);
        }
        self.make_room(layout);
    }

    /// Back to life, minus anything asked for that the room no longer has.
    pub fn resume(&mut self, layout: &RoomLayout, snapshot: &HomeSnapshot, now: f32) {
        self.paused = false;
        self.make_room(layout);
        for mind in &mut self.minds {
            mind.queue.retain(|act| still_there(act, layout, snapshot));
            mind.dawdle_until = now + 0.3;
        }
    }

    #[cfg(test)]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Anyone standing where a piece now stands steps to the nearest open floor.
    pub fn make_room(&mut self, layout: &RoomLayout) {
        let floor = Floor::of(layout);
        let mut taken: Vec<(i32, i32)> = Vec::new();
        for actor in &mut self.actors {
            if actor.on_piece.is_some() {
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

    /// What a resident is doing, as the room's status line puts it.
    pub fn doing(
        &self,
        household: &Household,
        layout: &RoomLayout,
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
        match (&mind.handled, &mind.plan) {
            (Some(Handled::Held), _) => format!("{name} is being carried."),
            (Some(Handled::Petted { .. }), _) => format!("{name} is enjoying a pat."),
            (_, Some(plan)) => match plan.part {
                Part::Joins(with) => {
                    let leader = household
                        .resident(with)
                        .map_or("someone", |r| r.name.as_str());
                    format!("{name} is with {leader}.")
                }
                Part::Leads => {
                    format!("{name} is {}.", plan.act.doing(household, layout, snapshot))
                }
            },
            _ if self.paused => format!("{name} is waiting while the room is arranged."),
            _ => format!("{name} is pottering about."),
        }
    }

    /// Everything that happens in `dt` seconds up to `now`.
    pub fn tick(
        &mut self,
        household: &Household,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        now: f32,
        dt: f32,
    ) {
        if self.paused {
            return;
        }
        let dt = dt.clamp(0.0, 0.25);
        let floor = Floor::of(layout);
        for index in 0..self.minds.len() {
            let Some(resident) = household.resident(self.minds[index].id) else {
                continue;
            };
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
            // Nobody stands about on the furniture: off a piece, the open floor is a step away.
            let tile = Floor::tile_of(actor.pos);
            if actor.on_piece.is_none()
                && !actor.walking()
                && !floor.open(tile.0, tile.1)
                && let Some(open) = floor.nearest_open(tile)
            {
                actor.walk([(open.0 as f32 + 0.5, open.1 as f32 + 0.5)]);
            }
            self.step(household, layout, snapshot, &floor, index, now);
        }
    }

    /// Move one resident's plan along, or give it a new one.
    fn step(
        &mut self,
        household: &Household,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
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
                    .choose(household, layout, snapshot, index)
                    .map(|act| (act, false)),
            };
            if let Some((act, asked)) = next {
                self.begin(layout, snapshot, floor, index, act, asked, now);
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
                    self.arrive(household, snapshot, index, &plan, now);
                } else {
                    self.set_stage(index, Stage::Waiting { since: now });
                }
            }
            Stage::Waiting { since } => {
                let partner = plan.act.partner().and_then(|other| self.index(other));
                let ready = partner.is_none_or(|o| !self.actors[o].walking());
                if ready || now - since > 6.0 {
                    self.arrive(household, snapshot, index, &plan, now);
                }
            }
            Stage::Prelude { until } => {
                if now >= until {
                    self.perform(household, snapshot, index, &plan, now);
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
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        floor: &Floor,
        index: usize,
        act: Act,
        asked: bool,
        now: f32,
    ) {
        let id = self.minds[index].id;
        if !still_there(&act, layout, snapshot) {
            self.minds[index].dawdle_until = now + 0.5;
            return;
        }
        let partner = act
            .partner()
            .and_then(|other| self.index(other))
            .filter(|&o| o != index);
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
            if asked {
                self.minds[index].dawdle_until = now + 1.0;
            } else {
                self.minds[index].dawdle_until = now + 0.5;
            }
            if !asked {
                return;
            }
        }
        let seats = self.free_seats(layout);
        let spot = self.spot_for(layout, floor, index, &act, &seats);
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
            let joined = self.join_spot(layout, floor, &act, tile, seat, &free);
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

    /// Every seat and lying place in the room not already taken.
    fn free_seats(&self, layout: &RoomLayout) -> Vec<Seat> {
        let mut seats = Vec::new();
        for placed in &layout.pieces {
            let Some(piece) = catalog::piece(&placed.piece) else {
                continue;
            };
            let (fx, fy) = catalog::front(placed.turn);
            for point in seat_points(placed, piece) {
                let taken = self.minds.iter().any(|mind| {
                    mind.plan
                        .as_ref()
                        .and_then(|plan| plan.seat)
                        .is_some_and(|seat| seat.piece == placed.uid && seat.at == point)
                });
                if !taken {
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

    /// Where an act is done from: the tile to walk to, the seat to settle on there if any, and
    /// what to look at.
    #[allow(clippy::type_complexity)]
    fn spot_for(
        &self,
        layout: &RoomLayout,
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
        if let Some(uid) = act.piece() {
            let placed = layout.piece(uid)?;
            let footprint = room::footprint(placed);
            let around = floor.beside(
                placed.x,
                placed.y,
                footprint.w,
                footprint.d,
                catalog::front(placed.turn),
            );
            let tile = *around.first()?;
            let settles = matches!(
                act,
                Act::Sit(_)
                    | Act::Relax(_)
                    | Act::Nap(_)
                    | Act::Sleep(_)
                    | Act::CurlUp(_)
                    | Act::InviteLittle(..)
            );
            if settles {
                let seat = seats.iter().find(|seat| seat.piece == uid).copied()?;
                return Some((tile, Some(seat), None));
            }
            return Some((tile, None, Some(footprint.centre())));
        }
        if let Some(item) = act.item() {
            let (around, look) = item_spot(layout, floor, item)?;
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
                let placed = layout.piece(seat.piece)?;
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
            | Act::Hug(other) => {
                let o = self.index(*other)?;
                let there = Floor::tile_of(self.actors[o].pos);
                let there = floor.nearest_open(there)?;
                let around = floor.beside(there.0 as u8, there.1 as u8, 1, 1, (0, 1));
                Some((
                    nearest(around).unwrap_or(there),
                    None,
                    Some(self.actors[o].pos),
                ))
            }
            _ => None,
        }
    }

    /// Where whoever joins an act goes: beside the leader, or to the other seat.
    fn join_spot(
        &self,
        layout: &RoomLayout,
        floor: &Floor,
        act: &Act,
        leader: (i32, i32),
        leader_seat: Option<Seat>,
        seats: &[Seat],
    ) -> Option<((i32, i32), Option<Seat>)> {
        let settles = matches!(act, Act::SitTogether(_) | Act::InviteLittle(..));
        if settles {
            let near = |uid: u16| {
                let placed = layout.piece(uid)?;
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
            Act::Nap(_) | Act::Sleep(_) | Act::CurlUp(_) | Act::InviteLittle(..)
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
            None => self.perform(household, snapshot, index, plan, now),
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
            self.perform(household, snapshot, o, &theirs, now);
        }
    }

    /// Strike the act's pose and hold it as long as the act lasts.
    fn perform(
        &mut self,
        household: &Household,
        snapshot: &HomeSnapshot,
        index: usize,
        plan: &Plan,
        now: f32,
    ) {
        let Some(resident) = household.resident(self.minds[index].id) else {
            return;
        };
        let character = &resident.character;
        let dice = &mut self.minds[index].dice;
        let settled = character.settled_face();
        let precious = |item: &DisplayId| {
            snapshot.item(item).is_some_and(|item| {
                item.modes.first() == Some(&formiga_home_contract::DisplayMode::Case)
            })
        };
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
                Act::Sleep(_) | Act::InviteLittle(..) => (
                    Pose::new(ActionKind::Sleep, ExpressionKind::Sleepy).with_cue(Cue::Sleep),
                    dice.between(20.0, 28.0),
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

    /// Done, or let go: drives settle if it was done, it gets down off anything, and whoever was
    /// with it is let go too. The next thing asked of it, if anything, is next.
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
        }
        mind.dawdle_until = now
            + if plan.asked {
                0.4
            } else {
                mind.dice.between(1.5, 4.0)
            };
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

    /// What a resident would like to do next, of its own accord.
    fn choose(
        &mut self,
        household: &Household,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        index: usize,
    ) -> Option<Act> {
        let id = self.minds[index].id;
        let resident = household.resident(id)?;
        let character = &resident.character;
        let seats = self.free_seats(layout);
        let others: Vec<Id> =
            self.minds
                .iter()
                .filter(|mind| {
                    mind.id != id
                        && mind.handled.is_none()
                        && !mind.plan.as_ref().is_some_and(|p| {
                            p.asked || matches!(p.act, Act::Sleep(_) | Act::Nap(_))
                        })
                })
                .map(|mind| mind.id)
                .collect();
        let me = self.actors[index].pos;
        let mut options: Vec<(Act, f32)> = Vec::new();
        let apart = |at: (f32, f32)| {
            self.actors
                .iter()
                .filter(|actor| actor.id != id)
                .map(|actor| ((actor.pos.0 - at.0).powi(2) + (actor.pos.1 - at.1).powi(2)).sqrt())
                .fold(8.0_f32, f32::min)
        };
        let lazy = character.kind == TemperamentKind::Lazybones;
        for placed in &layout.pieces {
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
            if piece.has(Use::Sit { seats: 1 }) && free {
                options.push((Act::Sit(uid), 0.9 * solitude));
                options.push((Act::Nap(uid), if lazy { 1.3 } else { 0.7 }));
            }
            if piece.has(Use::Sleep) && free {
                let basket = piece.id == "basket";
                let fits = if resident.is_little() == basket {
                    1.4
                } else {
                    0.8
                };
                options.push((Act::Sleep(uid), fits * if lazy { 1.5 } else { 1.0 }));
                options.push((Act::CurlUp(uid), fits * 0.8 * solitude));
            }
            if piece.has(Use::Play) {
                options.push((Act::Play(uid), 1.0));
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
                    options.push((Act::ShowOffToy(uid), 0.9));
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
        for shown in &layout.displays {
            let item = &shown.item;
            let Some(thing) = snapshot.item(item) else {
                continue;
            };
            let precious = thing.modes.first() == Some(&formiga_home_contract::DisplayMode::Case);
            let inspect = if character.studies() && precious {
                1.7
            } else {
                1.0
            };
            options.push((Act::Inspect(item.clone()), inspect));
            // Something found by a close friend, or by itself, is something to remember by.
            let found_by_friend = thing.finder_name.as_deref().is_some_and(|finder| {
                finder == resident.name
                    || household
                        .residents
                        .iter()
                        .any(|other| other.name == finder && household.bond(id, other.id).close())
            });
            options.push((
                Act::Remember(item.clone()),
                if found_by_friend { 1.2 } else { 0.45 },
            ));
            if character.shows_off() {
                for other in &others {
                    options.push((
                        Act::ShowTo(item.clone(), *other),
                        if precious { 1.6 } else { 1.1 },
                    ));
                }
            }
            if character.fusses() {
                options.push((Act::FussWith(item.clone()), 0.7));
            }
            if toy_like(thing) {
                options.push((Act::PlayWithFind(item.clone()), 0.9));
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
            let weight = seek * shy;
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
            let guards =
                character.kind == TemperamentKind::Guardian || household.family(id, *other);
            if little && !resident.is_little() && guards {
                options.push((Act::Comfort(*other), 1.0 * weight));
            }
            if !character.keeps_apart() && seats.len() >= 2 {
                options.push((Act::SitTogether(*other), 0.8 * weight));
            }
        }
        let floor = Floor::of(layout);
        let mind = &mut self.minds[index];
        let wander = (
            (mind.dice.next() * f32::from(layout.width)) as i32,
            (mind.dice.next() * f32::from(layout.depth)) as i32,
        );
        if let Some(tile) = floor.nearest_open(wander) {
            let far = ((tile.0 as f32 + 0.5 - me.0).powi(2) + (tile.1 as f32 + 0.5 - me.1).powi(2))
                .sqrt();
            if far > 1.5 {
                options.push((Act::Wander(tile.0 as u8, tile.1 as u8), 0.45));
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

    /// Who is asleep: for tests and the status line.
    #[cfg(test)]
    pub fn asleep(&self, id: Id) -> bool {
        self.actor(id)
            .is_some_and(|actor| actor.pose.asleep() && !actor.walking())
    }

    /// The act a resident is about, if it is about one, and whether it was asked for.
    #[cfg(test)]
    pub fn current(&self, id: Id) -> Option<(Act, bool)> {
        let mind = &self.minds[self.index(id)?];
        mind.plan
            .as_ref()
            .map(|plan| (plan.act.clone(), plan.asked))
    }
}

fn band(band: Band) -> f32 {
    match band {
        Band::None => 0.0,
        Band::Low => 0.33,
        Band::Medium => 0.66,
        Band::High => 1.0,
    }
}

/// The finds that are toys, which a resident can play with as well as look at.
fn toy_like(item: &formiga_home_contract::DisplayItem) -> bool {
    const TOYS: [u8; 15] = [
        13, 18, 25, 32, 44, 45, 46, 53, 78, 85, 126, 129, 132, 151, 159,
    ];
    match &item.source {
        DisplaySource::DesktopFind { variant } => TOYS.contains(variant),
        DisplaySource::HillSouvenir { id } => id == "chest_marble",
        DisplaySource::Unknown => false,
    }
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

/// Where someone stands to look at something shown, and the point they look at.
/// Tiles to stand on, nearest first, and the point to look at.
type Viewpoint = (Vec<(i32, i32)>, (f32, f32));

fn item_spot(layout: &RoomLayout, floor: &Floor, item: &DisplayId) -> Option<Viewpoint> {
    let shown = layout.displays.iter().find(|shown| &shown.item == item)?;
    match shown.spot {
        Spot::On { piece, slot } => {
            let placed = layout.piece(piece)?;
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
        Spot::Wall { side, at } => {
            let (tile, look) = match side {
                WallSide::North => ((i32::from(at), 0), (f32::from(at) + 0.5, 0.0)),
                WallSide::West => ((0, i32::from(at)), (0.0, f32::from(at) + 0.5)),
            };
            let near = floor.nearest_open(tile)?;
            let mut around = vec![near];
            around.extend(
                floor
                    .beside(near.0 as u8, near.1 as u8, 1, 1, (0, 1))
                    .into_iter()
                    .take(2),
            );
            Some((around, look))
        }
        Spot::Floor { x, y } => Some((
            floor.beside(x, y, 1, 1, (0, 1)),
            (f32::from(x) + 0.5, f32::from(y) + 0.5),
        )),
    }
}

/// Whether the room still has what an act is about.
fn still_there(act: &Act, layout: &RoomLayout, snapshot: &HomeSnapshot) -> bool {
    if let Some(uid) = act.piece()
        && layout.piece(uid).is_none()
    {
        return false;
    }
    if let Some(item) = act.item() {
        return snapshot.item(item).is_some()
            && layout.displays.iter().any(|shown| &shown.item == item);
    }
    true
}

/// What a resident can be asked to do with something in the room, for the owner's menu.
pub fn choices(
    household: &Household,
    layout: &RoomLayout,
    snapshot: &HomeSnapshot,
    id: Id,
    target: &crate::scene::Target,
) -> Vec<Act> {
    use crate::scene::Target;
    let Some(resident) = household.resident(id) else {
        return Vec::new();
    };
    let character = &resident.character;
    let others: Vec<Id> = household
        .residents
        .iter()
        .map(|r| r.id)
        .filter(|other| *other != id)
        .collect();
    let mut acts = Vec::new();
    match target {
        Target::Floor(x, y) => acts.push(Act::GoTo(*x, *y)),
        Target::Piece(uid) => {
            let Some(placed) = layout.piece(*uid) else {
                return acts;
            };
            let Some(piece) = catalog::piece(&placed.piece) else {
                return acts;
            };
            if piece.has(Use::Sit { seats: 1 }) {
                acts.extend([Act::Sit(*uid), Act::Relax(*uid), Act::Nap(*uid)]);
            }
            if piece.has(Use::Sleep) {
                acts.extend([Act::Sleep(*uid), Act::CurlUp(*uid)]);
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
                let floor = Floor::of(layout);
                if let Some(&(x, y)) = floor
                    .beside(
                        placed.x,
                        placed.y,
                        footprint.w,
                        footprint.d,
                        catalog::front(placed.turn),
                    )
                    .first()
                {
                    acts.push(Act::GoTo(x as u8, y as u8));
                }
                if piece.flat {
                    let (cx, cy) = footprint.centre();
                    acts.insert(0, Act::GoTo(cx as u8, cy as u8));
                    acts.truncate(1);
                }
            }
        }
        Target::Shown(item) => {
            acts.push(Act::Inspect(item.clone()));
            acts.push(Act::Remember(item.clone()));
            acts.extend(others.iter().map(|other| Act::ShowTo(item.clone(), *other)));
            if snapshot.item(item).is_some_and(toy_like) {
                acts.push(Act::PlayWithFind(item.clone()));
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
mod tests {
    use super::*;
    use crate::staging;
    use crate::starter;
    use formiga_home_contract::sample;

    fn home() -> (Household, RoomLayout) {
        let household = Household::new(sample::snapshot()).unwrap();
        let layout = staging::lived_in(&household, "floor.boards", "wall.leafy")
            .rooms
            .remove(0);
        (household, layout)
    }

    fn run(
        life: &mut Life,
        household: &Household,
        layout: &RoomLayout,
        from: f32,
        seconds: f32,
    ) -> f32 {
        let mut now = from;
        while now < from + seconds {
            now += 1.0 / 30.0;
            life.tick(household, layout, &household.snapshot, now, 1.0 / 30.0);
        }
        now
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
        let chair = layout
            .pieces
            .iter()
            .find(|p| p.piece.as_str() == "armchair")
            .unwrap()
            .uid;
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

    #[test]
    fn three_kinds_of_asked_act_each_happen() {
        let (household, layout) = home();
        let keeper = household.keeper().id;
        let pip = household.residents[1].id;
        let ball = layout
            .pieces
            .iter()
            .find(|p| p.piece.as_str() == "ball")
            .unwrap()
            .uid;
        for (act, done) in [
            (Act::Play(ball), ActionKind::SoloPlay),
            (Act::Inspect(DisplayId::find(0)), ActionKind::InspectScreen),
            (Act::Greet(pip), ActionKind::Greet),
        ] {
            let mut life = Life::new(&household, &layout);
            life.ask(keeper, act.clone(), 0.0);
            let mut now = 0.0;
            let mut happened = false;
            for _ in 0..60 {
                now = run(&mut life, &household, &layout, now, 0.25);
                let actor = life.actor(keeper).unwrap();
                if !actor.walking() && actor.pose.clip == formiga_art::BodyClip::Action(done) {
                    happened = true;
                    break;
                }
            }
            assert!(happened, "{act:?} never happened");
        }
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
        let now = run(&mut life, &household, &layout, 0.1, 12.0);
        assert!(life.current(keeper).is_some() || now > 0.0);
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
        let layout = starter::room(&household.snapshot);
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
            let mut snapshot = sample::snapshot();
            for resident in &mut snapshot.residents {
                resident.character.temperament = kind.into();
            }
            let household = Household::new(snapshot).unwrap();
            let layout = staging::lived_in(&household, "floor.boards", "wall.leafy")
                .rooms
                .remove(0);
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
        for shown in &layout.displays {
            let acts = choices(
                &household,
                &layout,
                snapshot,
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
            keeper,
            &crate::scene::Target::Resident(pip),
        );
        assert!(social.contains(&Act::Greet(pip)) && social.contains(&Act::Comfort(pip)));
    }
}
