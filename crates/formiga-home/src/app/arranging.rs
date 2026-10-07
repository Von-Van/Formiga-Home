//! The pointer on the room while arranging it: a piece, a find or a doorway picked up by a drag or
//! a click, and put down where it goes, drawn there first as it would look.

use super::*;
use crate::arrange::Landing;
use crate::placement::{Place, Showing};
use crate::scene::Ghost;

impl HomeApp {
    pub(super) fn arrange_pointer(
        &mut self,
        response: &egui::Response,
        pointer: Option<(f32, f32)>,
        released: bool,
    ) {
        if response.secondary_clicked() {
            self.arranging.turn();
            return;
        }
        // Picking something up from the house, by dragging it or by a click: a doorway first,
        // then what is pointed at.
        if self.arranging.carrying.is_none() && (response.drag_started() || response.clicked()) {
            let door = pointer
                .and_then(|point| arrange::wall_cell_at(&self.scene.view, &self.house, point))
                .filter(|wall| wall.door)
                .map(|wall| Carry::Door {
                    room: wall.room,
                    door: formiga_home_contract::Door {
                        side: wall.side,
                        at: wall.at,
                    },
                });
            let carry = door.or_else(|| match self.hovered.clone() {
                Some(Target::Piece(name)) => self.house.piece(name).map(|placed| Carry::Piece {
                    name,
                    turn: placed.turn,
                }),
                Some(Target::Shown(item)) => Some(Carry::Thing(item)),
                _ => None,
            });
            match carry {
                Some(carry) => {
                    self.arranging.carrying = Some(carry);
                    self.arranging.dragged = response.drag_started();
                }
                None if response.drag_started() => {
                    self.zoom.panning = self.zoom.closer_than_fits();
                }
                None => {}
            }
            return;
        }
        let Some(carrying) = self.arranging.carrying.clone() else {
            return;
        };
        let put_now = if self.arranging.dragged {
            released
        } else {
            response.clicked()
        };
        if !put_now {
            return;
        }
        let landing = pointer.and_then(|point| self.landing(point));
        let house = self.house.clone();
        let put = match landing {
            Some((landing, Some(_))) => self.arranging.put(
                &mut self.state,
                self.keeper,
                &self.household.snapshot,
                &house,
                landing,
            ),
            _ => false,
        };
        if put {
            let what = match carrying {
                Carry::Thing(_) => "Shown.",
                Carry::Door { .. } => "The doorway is moved.",
                _ => "Put down.",
            };
            self.changed(what);
        } else if pointer.is_some() {
            self.say("That will not go there.");
            if self.arranging.dragged {
                self.arranging.carrying = None;
            }
        } else {
            // Let go outside the room: it goes back where it came from.
            self.arranging.carrying = None;
        }
        self.arranging.dragged = false;
    }

    fn landing(&mut self, point: (f32, f32)) -> Option<(Landing, Option<Showing>)> {
        let now = self.now();
        let over = match &self.hovered {
            Some(Target::Piece(uid)) => Some(*uid),
            _ => None,
        };
        let over = over.or_else(|| {
            let pixel = (point.0 as i32, point.1 as i32);
            match self.scene.hit(
                &self.house,
                &self.household.snapshot,
                &mut self.life.actors,
                now,
                pixel,
            ) {
                Some(Target::Piece(uid)) => Some(uid),
                _ => None,
            }
        });
        self.arranging.landing(
            &self.scene.view,
            &self.house,
            home_of(&self.state, self.keeper),
            &self.household.snapshot,
            point,
            over,
        )
    }

    /// What is being carried, drawn where it would go: green where it fits, red where not. A
    /// doorway is shown as the stretch of wall it would go in.
    pub(super) fn ghost(
        &mut self,
        pointer: Option<(f32, f32)>,
    ) -> (Option<Ghost>, Option<(crate::house::Wall, bool)>) {
        let Some(point) = pointer else {
            return (None, None);
        };
        let Some((landing, showing)) = self.landing(point) else {
            return (None, None);
        };
        let house = self.house.clone();
        if let Landing::Wall { room, door } = landing {
            let wall = house.wall(room, door.side, door.at).copied();
            return (None, wall.map(|wall| (wall, showing.is_some())));
        }
        if let Some((piece, turn, _)) = self.arranging.carried_piece(&house) {
            let Landing::Floor { room, x, y } = landing else {
                return (None, None);
            };
            let Some(at) = house.room(room) else {
                return (None, None);
            };
            let (x, y) = (at.x + x, at.y + y);
            let (w, d) = piece.size_at(turn);
            let sprite = self.scene.piece_sprite(piece, turn).clone();
            return (
                Some(Ghost {
                    sprite,
                    at: self.scene.view.pixel(f32::from(x), f32::from(y)),
                    fits: showing.is_some(),
                    footprint: Some(placement::Footprint { x, y, w, d }),
                }),
                None,
            );
        }
        let Some(Carry::Thing(id)) = &self.arranging.carrying else {
            return (None, None);
        };
        let Some(item) = self.household.snapshot.item(id).cloned() else {
            return (None, None);
        };
        let Landing::Spot { room, spot } = landing else {
            return (None, None);
        };
        let Some(at) = house.at(room, spot) else {
            return (None, None);
        };
        let place = house.place_of(at).unwrap_or(Place::Top);
        let anchor = self
            .scene
            .spot_anchor(&house, at)
            .unwrap_or((point.0 as i32, point.1 as i32));
        let sprite = self
            .scene
            .thing(&item, place, showing.unwrap_or(Showing::Card))
            .clone();
        let footprint = match at {
            At::Floor { x, y } => Some(placement::Footprint { x, y, w: 1, d: 1 }),
            _ => None,
        };
        (
            Some(Ghost {
                sprite,
                at: anchor,
                fits: showing.is_some(),
                footprint,
            }),
            None,
        )
    }
}
