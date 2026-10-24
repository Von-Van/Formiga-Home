//! Visitors: friends Desktop has lent for the visit. They knock a little while after the house
//! opens and are answered in character, stay a few minutes, and go home the way they came, unless
//! asked to stay over, or minded to ask themselves.

use super::{Act, Event, Life, Presence, Seat, band, door, seats_for, way};
use crate::catalog::{self, Use};
use crate::house::House;
use crate::household::{Household, Id};
use crate::path::Floor;
use formiga_core::TemperamentKind;
use formiga_travel::Band;

impl Life {
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
    pub(super) fn stay_over(&mut self, index: usize) {
        let mind = &mut self.minds[index];
        mind.staying = true;
        mind.presence = Presence::Visiting {
            until: f32::INFINITY,
        };
        self.events.push(Event::StayingOver(mind.id));
    }

    /// Someone at the door: in it comes. A visitor is answered by the household in character;
    /// a resident back from the desktop is simply home.
    pub(super) fn knock(
        &mut self,
        household: &Household,
        house: &House,
        floor: &Floor,
        index: usize,
        now: f32,
    ) {
        let (outside, inside) = door(house, floor);
        let mind = &mut self.minds[index];
        let visitor = mind.id;
        let visiting = household.is_visitor(visitor);
        let stay = mind.dice.between(150.0, 210.0);
        mind.presence = if visiting {
            Presence::Visiting { until: now + stay }
        } else {
            Presence::Home
        };
        mind.dawdle_until = now + 3.0;
        let actor = &mut self.actors[index];
        actor.hidden = false;
        actor.pos = outside;
        actor.walk([(inside.0 as f32 + 0.5, inside.1 as f32 + 0.5)]);
        if !visiting {
            return;
        }
        self.events.push(Event::Arrived(visitor));
        let snapshot = &household.snapshot;
        for resident in &household.residents {
            let Some(r) = self.index(resident.id) else {
                continue;
            };
            if !self.minds[r].presence.in_house() {
                continue;
            }
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
                let seats = seats_for(self.actors[r].body(), house, &self.free_seats(house));
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
    pub(super) fn go_home(&mut self, house: &House, floor: &Floor, index: usize, now: f32) {
        self.end(index, now, false);
        let (outside, inside) = door(house, floor);
        let actor = &mut self.actors[index];
        let mut route = way(floor, actor, inside);
        route.push(outside);
        actor.walk(route);
        self.minds[index].presence = Presence::Leaving;
        self.minds[index].queue.clear();
    }
}
