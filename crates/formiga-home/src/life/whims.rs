//! What a resident chooses to do for itself, whenever nothing is asked of it: everything the
//! room and the company put in mind, each weighed by who it is, what it has come to like and how
//! it gets on with whoever is there, and then the one it most feels like.

use super::choices::item_room;
use super::{
    Act, Kind, Life, Seat, band, favourites, is_precious, seats_for, shown_on, subject, suits,
    toy_like, wearable,
};
use crate::body::Body;
use crate::catalog::{self, Family, Use};
use crate::house::House;
use crate::household::{Household, Id, Resident};
use crate::path::Floor;
use crate::placement;
use formiga_core::{Habit, TemperamentKind};
use formiga_home_contract::{DisplayId, DisplayItem, DisplaySource, HomeSnapshot, Liked, Liking};
use formiga_travel::{Band, Trait};

/// How likely, once a room has palled, the next thing it thinks of doing is somewhere else.
const LEAVE_ROOM: f32 = 0.65;

impl Life {
    /// What someone would like to do next, of its own accord.
    pub(super) fn choose(
        &mut self,
        household: &Household,
        house: &House,
        snapshot: &HomeSnapshot,
        likings: &[Liking],
        index: usize,
    ) -> Option<Act> {
        let id = self.minds[index].id;
        let resident = household.resident(id)?;
        let choosing = Choosing {
            household,
            house,
            snapshot,
            likings,
            resident,
            index,
            visiting: household.is_visitor(id),
            seats: seats_for(self.actors[index].body(), house, &self.free_seats(house)),
            others: self.company(id),
            visitors_here: self.minds.iter().any(|mind| {
                mind.id != id && household.is_visitor(mind.id) && mind.presence.in_house()
            }),
            loved: favourites(likings, house, id),
        };
        let mut options: Vec<(Act, f32)> = Vec::new();
        for placed in &house.pieces {
            self.piece_whims(&choosing, placed, &mut options);
        }
        for shown in &house.shown {
            self.find_whims(&choosing, &shown.item, &mut options);
        }
        for other in &choosing.others {
            company_whims(&choosing, *other, &mut options);
        }
        suit_rooms(house, &mut options);
        let (room, leaving) = self.which_room(house, index);
        options.extend(self.look_round(house, index, room, leaving));
        keep_to_room(house, room, &mut options);
        self.wait_for_others_elsewhere(house, index, room, &mut options);
        let body = self.actors[index].body();
        options.retain(|(act, _)| suits(body, house, act));
        self.likeliest(index, resident.character.whimsy(), options)
    }

    /// Whoever else is in the house and free for company: not being handled, and not about
    /// something asked of it, or asleep.
    fn company(&self, id: Id) -> Vec<Id> {
        self.minds
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
            .collect()
    }

    /// How far a point is from everyone else in the house, up to a good way off.
    fn apart(&self, id: Id, at: (f32, f32)) -> f32 {
        self.actors
            .iter()
            .filter(|actor| actor.id != id && !actor.hidden)
            .map(|actor| ((actor.pos.0 - at.0).powi(2) + (actor.pos.1 - at.1).powi(2)).sqrt())
            .fold(8.0_f32, f32::min)
    }

    /// What a piece puts in mind to do with it.
    fn piece_whims(
        &self,
        choosing: &Choosing,
        placed: &formiga_home_contract::PlacedPiece,
        options: &mut Vec<(Act, f32)>,
    ) {
        let Some(piece) = catalog::piece(&placed.piece) else {
            return;
        };
        let uid = placed.uid;
        let centre = placement::footprint(placed).centre();
        let prospect = Prospect {
            piece,
            uid,
            free: choosing.seats.iter().any(|seat| seat.piece == uid),
            solitude: if choosing.resident.character.keeps_apart() {
                0.6 + self.apart(choosing.resident.id, centre) / 6.0
            } else {
                1.0
            },
            mine: choosing.piece_favourite(uid),
        };
        self.seat_whims(choosing, &prospect, options);
        self.bed_whims(choosing, &prospect, options);
        self.pastime_whims(choosing, &prospect, options);
        play_whims(choosing, &prospect, options);
    }

    /// A seat to sit or nap on, if there is room on it; and a grump's to grumble about, if
    /// somebody else is in its favourite.
    fn seat_whims(&self, choosing: &Choosing, prospect: &Prospect, options: &mut Vec<(Act, f32)>) {
        let Prospect {
            piece,
            uid,
            free,
            solitude,
            mine,
        } = *prospect;
        let kind = choosing.resident.character.kind;
        let lazy = kind == TemperamentKind::Lazybones;
        if piece.has(Use::Sit { seats: 1 }) {
            if free {
                options.push((Act::Sit(uid), 0.9 * solitude * mine));
                options.push((Act::Nap(uid), if lazy { 1.3 } else { 0.7 } * mine));
            } else if kind == TemperamentKind::Grump
                && mine > 1.0
                && let Some(sitter) = self
                    .on_piece(uid)
                    .filter(|sitter| *sitter != choosing.resident.id)
            {
                // Somebody is in its chair.
                options.push((Act::GrumbleAt(sitter), 1.6));
            }
        }
    }

    /// A bed to sleep, curl up or bounce on: one of the household's own, or for a friend staying
    /// over, a spare.
    fn bed_whims(&self, choosing: &Choosing, prospect: &Prospect, options: &mut Vec<(Act, f32)>) {
        let Prospect {
            piece,
            uid,
            free,
            solitude,
            mine,
        } = *prospect;
        let resident = choosing.resident;
        let character = &resident.character;
        let kind = character.kind;
        let lazy = kind == TemperamentKind::Lazybones;
        let visiting = choosing.visiting;
        let guest = visiting && self.minds[choosing.index].staying;
        if piece.has(Use::Sleep)
            && free
            && guest
            && spare_bed(choosing.household, choosing.house, choosing.likings, uid)
        {
            // A friend staying over sleeps in the guest bedroll if there is one, or in a bed
            // nobody at home has made their own.
            let bedroll = if piece.id == "bedroll" { 2.0 } else { 1.0 };
            options.push((Act::Sleep(uid), 1.2 * bedroll));
            options.push((Act::CurlUp(uid), 0.6 * bedroll * solitude));
        }
        if piece.has(Use::Sleep) && free && !visiting {
            let basket = piece.is_basket();
            let fits = if resident.is_little() == basket {
                1.4
            } else {
                0.8
            };
            let bedtime = 1.0 + self.dark * 3.0;
            options.push((
                Act::Sleep(uid),
                fits * if lazy { 1.5 } else { 1.0 } * mine * bedtime,
            ));
            options.push((Act::CurlUp(uid), fits * 0.8 * solitude * mine));
            if (character.playful() && kind == TemperamentKind::Troublemaker)
                || resident.is_little()
            {
                options.push((Act::Bounce(uid), 0.7));
            }
        }
    }

    /// The quieter things a piece is for: stretching out on a rug, sitting at a table or drawing
    /// there, tending a plant, switching a lamp, looking over a shelf.
    fn pastime_whims(
        &self,
        choosing: &Choosing,
        prospect: &Prospect,
        options: &mut Vec<(Act, f32)>,
    ) {
        let Prospect {
            piece,
            uid,
            solitude,
            ..
        } = *prospect;
        let resident = choosing.resident;
        let character = &resident.character;
        let lazy = character.kind == TemperamentKind::Lazybones;
        let visiting = choosing.visiting;
        if piece.flat {
            options.push((Act::Sprawl(uid), if lazy { 1.2 } else { 0.35 }));
        }
        if piece.family == Family::Tables && !visiting {
            options.push((Act::SitAt(uid), 0.5 * solitude));
            if resident.is_little() && !self.minds[choosing.index].drew {
                let of = subject(choosing.household, resident.id, &choosing.others);
                options.push((Act::Draw(uid, of), 0.5 + character.axes.curiosity * 0.4));
            }
        }
        if piece.family == Family::Plants && !visiting {
            options.push((Act::Tend(uid), 0.25 + character.axes.affection * 0.3));
        }
        if piece.has(Use::Warm) {
            // A fire draws everyone as the evening comes, the idle most of all.
            options.push((
                Act::WarmUp(uid),
                (0.3 + self.dark * 1.4) * if lazy { 1.4 } else { 1.0 } * solitude.max(0.6),
            ));
        } else if piece.family == Family::Lights && character.fusses() {
            options.push((Act::SwitchLamp(uid), 0.25));
        }
        if piece.has(Use::Wind) {
            // A clock is kept by whoever minds such things.
            let minds = if character.fusses() || character.studies() {
                0.6
            } else {
                0.15
            };
            options.push((Act::Wind(uid), minds));
        }
        if piece.family == Family::Shelves && !shown_on(choosing.house, uid).is_empty() {
            let lots = if character.studies() || visiting {
                1.0
            } else {
                0.4
            };
            options.push((Act::Browse(uid), lots));
        }
    }

    /// What a find on show puts in mind: a good look, a memory, showing it off, fussing with it,
    /// playing with it, trying it on. Not one somebody is wearing just now.
    fn find_whims(&self, choosing: &Choosing, item: &DisplayId, options: &mut Vec<(Act, f32)>) {
        let Some(thing) = choosing.snapshot.item(item) else {
            return;
        };
        if self.worn.iter().any(|(worn, _)| worn == item) {
            return;
        }
        let character = &choosing.resident.character;
        let visiting = choosing.visiting;
        let precious = is_precious(thing);
        let souvenir = matches!(thing.source, DisplaySource::HillSouvenir { .. });
        let mine = if choosing.favourite(&Liked::Shown { item: item.clone() }) {
            1.6
        } else {
            1.0
        };
        let inspect = self.inspect_weight(choosing, item, precious, souvenir);
        options.push((Act::Inspect(item.clone()), inspect * mine));
        // Something found by a close friend, or by itself, is something to remember by.
        let found_by_friend = found_by_friend(choosing, thing);
        options.push((
            Act::Remember(item.clone()),
            if found_by_friend { 1.2 } else { 0.45 } * mine,
        ));
        if character.shows_off() && !visiting {
            show_off_whims(choosing, item, precious, souvenir, mine, options);
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
                    character.kind,
                    TemperamentKind::Oddball | TemperamentKind::Troublemaker
                ) {
                0.7
            } else {
                0.2
            };
            options.push((Act::TryOn(item.clone()), dressy));
        }
    }

    /// How much a find on show calls for a good look: most to one who studies things, if it is
    /// precious; to a visitor, if it has not seen it yet; and to the inquisitive, if it is new.
    fn inspect_weight(
        &self,
        choosing: &Choosing,
        item: &DisplayId,
        precious: bool,
        souvenir: bool,
    ) -> f32 {
        let character = &choosing.resident.character;
        let mut inspect = if character.studies() && precious {
            1.7
        } else {
            1.0
        };
        // A visitor takes a good look at everything on show it has not seen yet, and most of
        // all at whatever stands out.
        if choosing.visiting && !self.minds[choosing.index].seen.contains(item) {
            inspect *= if precious || souvenir { 2.4 } else { 1.8 };
        }
        // Something new since the room was arranged draws the inquisitive first.
        if self.novel.contains(item)
            && (character.studies() || character.kind == TemperamentKind::Oddball)
        {
            inspect *= 2.2;
        }
        inspect
    }

    /// Which room to be in: the one it is in, until that palls, and then, more often than not,
    /// one of the others, however little there is to do there. And whether it is leaving this
    /// one for it.
    fn which_room(&mut self, house: &House, index: usize) -> (u8, bool) {
        let here = house.room_of_point(self.actors[index].pos);
        let mind = &mut self.minds[index];
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
        (room, leaving)
    }

    /// A look round somewhere in `room`, if there is open floor there a few steps off; the more
    /// likely for leaving the room it is in.
    fn look_round(
        &mut self,
        house: &House,
        index: usize,
        room: u8,
        leaving: bool,
    ) -> Option<(Act, f32)> {
        let me = self.actors[index].pos;
        let floor = self.floor(house);
        let dice = &mut self.minds[index].dice;
        let area = &house.rooms[usize::from(room)];
        let wander = (
            i32::from(area.x) + (dice.next() * f32::from(area.width)) as i32,
            i32::from(area.y) + (dice.next() * f32::from(area.depth)) as i32,
        );
        let tile = floor.nearest_open_in(wander, room)?;
        let far =
            ((tile.0 as f32 + 0.5 - me.0).powi(2) + (tile.1 as f32 + 0.5 - me.1).powi(2)).sqrt();
        (far > 1.5).then(|| {
            let weight = if leaving { 1.2 } else { 0.5 };
            (Act::Wander(tile.0 as u8, tile.1 as u8), weight)
        })
    }

    /// Someone in another room can wait a while, for one only just come into this one.
    fn wait_for_others_elsewhere(
        &self,
        house: &House,
        index: usize,
        room: u8,
        options: &mut [(Act, f32)],
    ) {
        let rooms_now: Vec<(Id, Option<u8>)> = self
            .minds
            .iter()
            .zip(&self.actors)
            .map(|(mind, actor)| (mind.id, house.room_of_point(actor.pos)))
            .collect();
        let settling = 0.35 + self.minds[index].restless * 0.65;
        for (act, weight) in options {
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
    }

    /// Of everything that came to mind, what it most feels like: whatever answers its strongest
    /// drive, unless it did that last, give or take as much whim as it has.
    fn likeliest(&mut self, index: usize, whimsy: f32, options: Vec<(Act, f32)>) -> Option<Act> {
        let mind = &mut self.minds[index];
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
}

/// What a resident choosing for itself has to go on.
struct Choosing<'a> {
    household: &'a Household,
    house: &'a House,
    snapshot: &'a HomeSnapshot,
    likings: &'a [Liking],
    resident: &'a Resident,
    index: usize,
    visiting: bool,
    seats: Vec<Seat>,
    /// Whoever else is about and free for company.
    others: Vec<Id>,
    /// Whether a visitor is in, besides itself.
    visitors_here: bool,
    loved: Vec<(Kind, Liked)>,
}

impl Choosing<'_> {
    fn favourite(&self, thing: &Liked) -> bool {
        self.loved.iter().any(|(_, liked)| liked == thing)
    }

    /// How much more a piece appeals for being a favourite.
    fn piece_favourite(&self, uid: u16) -> f32 {
        if self
            .house
            .liked(uid)
            .is_some_and(|thing| self.favourite(&thing))
        {
            1.8
        } else {
            1.0
        }
    }
}

/// A piece as someone choosing for itself sees it: whether it has a place free, how far from
/// everyone else it is to one who keeps apart, and how much more it appeals for being a
/// favourite.
#[derive(Clone, Copy)]
struct Prospect<'a> {
    piece: &'a catalog::Piece,
    uid: u16,
    free: bool,
    solitude: f32,
    mine: f32,
}

/// A toy to play with, alone, with someone or to show off with; and a snack.
fn play_whims(choosing: &Choosing, prospect: &Prospect, options: &mut Vec<(Act, f32)>) {
    let Prospect {
        piece, uid, mine, ..
    } = *prospect;
    let household = choosing.household;
    let resident = choosing.resident;
    let character = &resident.character;
    if piece.has(Use::Play) {
        options.push((Act::Play(uid), 1.0 * mine));
        for other in &choosing.others {
            let bond = household.bond(resident.id, *other);
            if character.playful() && bond.friction < Band::High {
                options.push((
                    Act::PlayWith(uid, *other),
                    0.6 + band(bond.playfulness) * 0.6,
                ));
            }
        }
        if character.shows_off() && !choosing.others.is_empty() {
            options.push((
                Act::ShowOffToy(uid),
                if choosing.visitors_here { 1.5 } else { 0.9 },
            ));
        }
    }
    if piece.has(Use::Show) {
        // A show-off puts on a show sooner, and sooner still with someone to watch.
        let keen = if character.shows_off() { 1.4 } else { 0.6 };
        options.push((Act::PutOnShow(uid), keen * mine));
        for other in &choosing.others {
            let bond = household.bond(resident.id, *other);
            if bond.friction < Band::High {
                options.push((
                    Act::PutOnShowFor(uid, *other),
                    keen * (0.5 + band(bond.playfulness) * 0.6) * mine,
                ));
            }
        }
    }
    let loves_food = resident
        .traveler
        .character
        .trait_ids
        .contains(&Trait::FoodMotivated)
        || character.has_habit(Habit::LooksFoodOver);
    if piece.has(Use::Snack) {
        options.push((Act::Snack(uid), if loves_food { 1.5 } else { 0.6 }));
    }
    if piece.has(Use::Supper) {
        options.push((Act::Supper(uid), if loves_food { 1.4 } else { 0.7 }));
    }
}

/// Whether a find was found by whoever is choosing, or by a close friend of its.
fn found_by_friend(choosing: &Choosing, thing: &DisplayItem) -> bool {
    let household = choosing.household;
    let resident = choosing.resident;
    thing.finder_name.as_deref().is_some_and(|finder| {
        finder == resident.name
            || household
                .everyone()
                .any(|other| other.name == finder && household.bond(resident.id, other.id).close())
    })
}

/// A show-off's find to show to each of the others: its best audience a visitor, and its best
/// find the oddest.
fn show_off_whims(
    choosing: &Choosing,
    item: &DisplayId,
    precious: bool,
    souvenir: bool,
    mine: f32,
    options: &mut Vec<(Act, f32)>,
) {
    for other in &choosing.others {
        let audience = if choosing.household.is_visitor(*other) {
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

/// What someone else in the house puts in mind: a hello, a hug, a game, some teasing, comfort
/// for a little one, a seat together. Most of all with a close friend or family, and with
/// whoever is visiting.
fn company_whims(choosing: &Choosing, other: Id, options: &mut Vec<(Act, f32)>) {
    let household = choosing.household;
    let resident = choosing.resident;
    let id = resident.id;
    let character = &resident.character;
    let bond = household.bond(id, other);
    let family = household.family(id, other);
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
    let company = if choosing.visiting || household.is_visitor(other) {
        1.4
    } else {
        1.0
    };
    let weight = seek * shy * company;
    options.push((Act::Greet(other), 0.5 * weight));
    if character.hugs() && (bond.warmth >= Band::Medium || family) {
        options.push((Act::Hug(other), 0.9 * weight));
    }
    if character.playful() {
        options.push((
            Act::PlayTogether(other),
            (0.5 + band(bond.playfulness) * 0.6) * weight,
        ));
    }
    if character.teases() {
        options.push((Act::Tease(other), 0.8));
    }
    let little = household.resident(other).is_some_and(|r| r.is_little());
    let guards = character.kind == TemperamentKind::Guardian || household.family(id, other);
    if little && !resident.is_little() && guards {
        options.push((Act::Comfort(other), 1.0 * weight));
    }
    // A seat together, if the other can get up onto one of them too.
    let their_seat = household.resident(other).is_some_and(|them| {
        let body = Body::of(them.genome());
        choosing
            .seats
            .iter()
            .any(|seat| body.settles_on_placed(choosing.house, seat.piece))
    });
    if !character.keeps_apart() && choosing.seats.len() >= 2 && their_seat {
        options.push((Act::SitTogether(other), 0.8 * weight));
    }
}

/// What a room is for comes the more readily to mind there: a nap in the bedroom, a game in the
/// playroom, a snack in the kitchen.
pub(super) fn suit_rooms(house: &House, options: &mut [(Act, f32)]) {
    use catalog::Purpose;
    for (act, weight) in options {
        let room = match (act.piece(), act.item()) {
            (Some(uid), _) => house
                .piece(uid)
                .and_then(|placed| house.room_of_point(placement::footprint(placed).centre())),
            (_, Some(item)) => item_room(house, item),
            (None, None) => None,
        };
        let Some(purpose) = room
            .and_then(|room| house.rooms.get(usize::from(room)))
            .and_then(|room| room.kind.as_ref())
            .and_then(catalog::template)
            .map(|template| template.purpose)
        else {
            continue;
        };
        let suits = match purpose {
            Purpose::Reading => matches!(act, Act::Browse(_) | Act::Inspect(_) | Act::Remember(_)),
            Purpose::Rest => matches!(
                act,
                Act::Sleep(_) | Act::Nap(_) | Act::CurlUp(_) | Act::InviteLittle(..)
            ),
            Purpose::Looking => matches!(
                act,
                Act::Browse(_) | Act::Inspect(_) | Act::Remember(_) | Act::ShowTo(..)
            ),
            Purpose::Play => matches!(
                act,
                Act::Play(_)
                    | Act::PlayWith(..)
                    | Act::PlayWithFind(_)
                    | Act::ShowOffToy(_)
                    | Act::PutOnShow(_)
                    | Act::PutOnShowFor(..)
            ),
            Purpose::Eating => matches!(act, Act::Snack(_) | Act::Share(..) | Act::Supper(_)),
            Purpose::Writing => matches!(act, Act::SitAt(_) | Act::Draw(..) | Act::Wind(_)),
        };
        if suits {
            *weight *= 1.5;
        }
    }
}

/// Only what is in `room` comes to mind, and the others, wherever they are: if anything there
/// does.
fn keep_to_room(house: &House, room: u8, options: &mut Vec<(Act, f32)>) {
    let in_room = |act: &Act| {
        let there = match (act.piece(), act.item()) {
            (Some(uid), _) => house
                .piece(uid)
                .and_then(|placed| house.room_of_point(placement::footprint(placed).centre())),
            (_, Some(item)) => item_room(house, item),
            (None, None) => return true,
        };
        there == Some(room)
    };
    if options.iter().any(|(act, _)| in_room(act)) {
        options.retain(|(act, _)| in_room(act));
    }
}

/// Whether a bed is free for a guest: nobody at home has made it their favourite.
fn spare_bed(household: &Household, house: &House, likings: &[Liking], uid: u16) -> bool {
    !household.residents.iter().any(|resident| {
        favourites(likings, house, resident.id)
            .iter()
            .any(|(kind, liked)| *kind == Kind::Bed && house.of_liked(liked) == Some(uid))
    })
}
