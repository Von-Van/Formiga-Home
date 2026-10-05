//! Arrange Mode: picking things up, carrying them about the room, putting them down where they
//! fit, and putting them back in the catalogue. Nothing is ever sold or used up: a piece put away
//! is simply not in the room, and a find put away is back in the drawer.
//!
//! Every change can be undone. What is undone is every home at once, since moving a find from
//! another house into this one changes both.

use crate::catalog::{self, Piece};
use crate::iso::{View, WALL_HEIGHT};
use crate::room::{self, Showing};
use formiga_home_contract::{
    DisplayId, HomeSnapshot, HomeState, HouseholdHome, RoomLayout, Spot, TravelerId, WallSide,
};

/// How many changes can be undone.
const HISTORY: usize = 40;

/// What is being carried.
#[derive(Clone, Debug, PartialEq)]
pub enum Carry {
    /// A piece from the catalogue, at a turn.
    New { piece: &'static Piece, turn: u8 },
    /// A piece already in the room, lifted out of it while it is carried.
    Piece { uid: u16, turn: u8 },
    /// Something the colony has, from this house, another, or the drawer.
    Thing(DisplayId),
}

/// Where a carried thing would go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Landing {
    Floor { x: u8, y: u8 },
    Spot(Spot),
}

#[derive(Default)]
pub struct Arranging {
    pub carrying: Option<Carry>,
    /// Carried by holding the pointer down, and put down where it is let go.
    pub dragged: bool,
    undo: Vec<HomeState>,
    redo: Vec<HomeState>,
}

impl Arranging {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Remember the homes as they are, before a change.
    pub fn remember(&mut self, state: &HomeState) {
        self.undo.push(state.clone());
        if self.undo.len() > HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, state: &mut HomeState) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(state, before));
        self.carrying = None;
        true
    }

    pub fn redo(&mut self, state: &mut HomeState) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(state, after));
        self.carrying = None;
        true
    }

    /// A quarter turn for whatever piece is being carried.
    pub fn turn(&mut self) {
        if let Some(Carry::New { turn, .. } | Carry::Piece { turn, .. }) = &mut self.carrying {
            *turn = (*turn + 1) % 4;
        }
    }

    /// The piece being carried, and its turn.
    pub fn carried_piece(&self, layout: &RoomLayout) -> Option<(&'static Piece, u8, Option<u16>)> {
        match &self.carrying {
            Some(Carry::New { piece, turn }) => Some((piece, *turn, None)),
            Some(Carry::Piece { uid, turn }) => {
                let placed = layout.piece(*uid)?;
                Some((catalog::piece(&placed.piece)?, *turn, Some(*uid)))
            }
            _ => None,
        }
    }

    /// Where what is carried would go with the pointer at `point` on the scene, and how it would
    /// show; `None` for the showing when it cannot go there.
    pub fn landing(
        &self,
        view: &View,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        point: (f32, f32),
        over_piece: Option<u16>,
    ) -> Option<(Landing, Option<Showing>)> {
        if let Some((piece, turn, moving)) = self.carried_piece(layout) {
            let (w, d) = piece.size_at(turn);
            let (fx, fy) = view.floor_at(point.0, point.1);
            // The pointer holds the piece by its middle.
            let x = (fx - f32::from(w) / 2.0 + 0.5).floor();
            let y = (fy - f32::from(d) / 2.0 + 0.5).floor();
            let x = x.clamp(0.0, f32::from(layout.width.saturating_sub(w))) as u8;
            let y = y.clamp(0.0, f32::from(layout.depth.saturating_sub(d))) as u8;
            let fits = room::can_place(layout, piece, x, y, turn, moving);
            return Some((Landing::Floor { x, y }, fits.then_some(Showing::Itself)));
        }
        let Some(Carry::Thing(id)) = &self.carrying else {
            return None;
        };
        let item = snapshot.item(id)?;
        // On a piece's surfaces, the free one nearest the pointer.
        if let Some(uid) = over_piece
            && let Some(placed) = layout.piece(uid)
        {
            let best = room::surfaces(placed)
                .into_iter()
                .filter_map(|surface| {
                    let spot = Spot::On {
                        piece: uid,
                        slot: surface.slot,
                    };
                    let showing = room::can_show(layout, item, spot)?;
                    let (sx, sy) = view.screen(surface.at.0, surface.at.1);
                    let distance =
                        (sx - point.0).powi(2) + (sy - surface.height as f32 - point.1).powi(2);
                    Some((spot, showing, distance))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((spot, showing, _)) = best {
                return Some((Landing::Spot(spot), Some(showing)));
            }
        }
        if let Some(spot) = wall_at(view, point) {
            return Some((Landing::Spot(spot), room::can_show(layout, item, spot)));
        }
        let (x, y) = view.tile_at(point.0, point.1)?;
        let spot = Spot::Floor { x, y };
        Some((Landing::Spot(spot), room::can_show(layout, item, spot)))
    }

    /// Put what is carried down at `landing`, if it fits. Whether it was put down.
    pub fn put(
        &mut self,
        state: &mut HomeState,
        keeper: TravelerId,
        snapshot: &HomeSnapshot,
        landing: Landing,
    ) -> bool {
        let Some(carrying) = self.carrying.clone() else {
            return false;
        };
        let before = state.clone();
        let Some(home) = state.household_mut(keeper) else {
            return false;
        };
        let done = match (&carrying, landing) {
            (Carry::New { piece, turn }, Landing::Floor { x, y }) => {
                room::has_room_for_more(home)
                    && room::can_place(&home.rooms[0], piece, x, y, *turn, None)
                    && {
                        room::add_piece(&mut home.rooms[0], piece, x, y, *turn);
                        true
                    }
            }
            (Carry::Piece { uid, turn }, Landing::Floor { x, y }) => {
                let layout = &mut home.rooms[0];
                let fits = layout
                    .piece(*uid)
                    .and_then(|placed| catalog::piece(&placed.piece))
                    .is_some_and(|piece| room::can_place(layout, piece, x, y, *turn, Some(*uid)));
                if fits
                    && let Some(placed) = layout.pieces.iter_mut().find(|placed| placed.uid == *uid)
                {
                    placed.x = x;
                    placed.y = y;
                    placed.turn = *turn;
                    // What it holds may no longer fit where it was shown, if it was turned.
                    true
                } else {
                    false
                }
            }
            (Carry::Thing(id), Landing::Spot(spot)) => {
                let fits = snapshot
                    .item(id)
                    .is_some_and(|item| room::can_show(&home.rooms[0], item, spot).is_some());
                let shown_here = home.shown().any(|shown| shown == id);
                if fits && (shown_here || room::has_room_for_more(home)) {
                    room::show(home, 0, id, spot);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if !done {
            return false;
        }
        // A find moved here from another house is no longer shown there.
        if let Carry::Thing(id) = &carrying {
            for other in &mut state.households {
                if other.keeper != keeper {
                    other.take_down(id);
                }
            }
        }
        self.remember(&before);
        self.carrying = None;
        self.dragged = false;
        true
    }

    /// Put what is carried back in the catalogue or the drawer: out of the room, and nothing lost.
    pub fn put_away(&mut self, state: &mut HomeState, keeper: TravelerId) -> bool {
        let Some(carrying) = self.carrying.take() else {
            return false;
        };
        self.dragged = false;
        let before = state.clone();
        let Some(home) = state.household_mut(keeper) else {
            return false;
        };
        let changed = match carrying {
            Carry::New { .. } => false,
            Carry::Piece { uid, .. } => {
                room::remove_piece(&mut home.rooms[0], uid);
                true
            }
            Carry::Thing(id) => home.take_down(&id),
        };
        if changed {
            self.remember(&before);
        }
        changed
    }

    /// Change the room's floor or walls.
    pub fn finish(
        &mut self,
        state: &mut HomeState,
        keeper: TravelerId,
        floor: Option<&str>,
        wall: Option<&str>,
    ) {
        let before = state.clone();
        let Some(home) = state.household_mut(keeper) else {
            return;
        };
        let layout = &mut home.rooms[0];
        let mut changed = false;
        if let Some(id) = floor.and_then(formiga_home_contract::CatalogId::parse)
            && layout.floor != id
        {
            layout.floor = id;
            changed = true;
        }
        if let Some(id) = wall.and_then(formiga_home_contract::CatalogId::parse)
            && layout.wall != id
        {
            layout.wall = id;
            changed = true;
        }
        if changed {
            self.remember(&before);
        }
    }
}

/// The stretch of wall under a point on the scene, if it is on one of the two walls, high enough
/// above the skirting to hang something.
pub fn wall_at(view: &View, point: (f32, f32)) -> Option<Spot> {
    let (ox, oy) = view.origin;
    let north = (point.0 - ox) / 16.0;
    let west = (ox - point.0) / 16.0;
    for (side, along, length) in [
        (WallSide::North, north, view.width),
        (WallSide::West, west, view.depth),
    ] {
        if along < 0.0 || along >= f32::from(length) {
            continue;
        }
        let floor_y = oy + along * 8.0;
        let up = floor_y - point.1;
        if (10.0..WALL_HEIGHT as f32 - 2.0).contains(&up) {
            return Some(Spot::Wall {
                side,
                at: along as u8,
            });
        }
    }
    None
}

/// The household's home in `state`, made if it has none yet.
pub fn ensure_home(state: &mut HomeState, snapshot: &HomeSnapshot) -> HouseholdHome {
    if state.household(snapshot.household.keeper).is_none() {
        state.set_household(crate::starter::home(snapshot));
    }
    state
        .household(snapshot.household.keeper)
        .cloned()
        .expect("the household's home was just made")
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::{HomeDocument, sample};

    fn setup() -> (HomeSnapshot, HomeState, View) {
        let snapshot = sample::snapshot();
        let mut state = HomeState::new(&snapshot.colony_key);
        ensure_home(&mut state, &snapshot);
        (snapshot, state, View::new(8, 8))
    }

    #[test]
    fn a_new_piece_goes_where_it_fits_and_can_be_undone_and_redone() {
        let (snapshot, mut state, view) = setup();
        let keeper = snapshot.household.keeper;
        let mut arranging = Arranging::default();
        let lamp = catalog::PIECES.iter().find(|p| p.id == "lamp").unwrap();
        arranging.carrying = Some(Carry::New {
            piece: lamp,
            turn: 0,
        });
        let layout = state.household(keeper).unwrap().rooms[0].clone();
        let point = view.tile_centre(6, 6);
        let (landing, showing) = arranging
            .landing(&view, &layout, &snapshot, point, None)
            .unwrap();
        assert_eq!(landing, Landing::Floor { x: 6, y: 6 });
        assert_eq!(showing, Some(Showing::Itself));
        let before = state.clone();
        assert!(arranging.put(&mut state, keeper, &snapshot, landing));
        assert_eq!(
            state.household(keeper).unwrap().rooms[0].pieces.len(),
            layout.pieces.len() + 1
        );
        assert!(arranging.undo(&mut state));
        assert_eq!(state, before);
        assert!(arranging.redo(&mut state));
        state.validate().unwrap();
    }

    #[test]
    fn a_find_dragged_onto_a_shelf_lands_on_the_shelf() {
        let (snapshot, mut state, view) = setup();
        let keeper = snapshot.household.keeper;
        let layout = state.household(keeper).unwrap().rooms[0].clone();
        let shelf = layout
            .pieces
            .iter()
            .find(|p| p.piece.as_str() == "shelf")
            .unwrap()
            .uid;
        let mut arranging = Arranging {
            carrying: Some(Carry::Thing(DisplayId::find(3))),
            ..Arranging::default()
        };
        let surface = room::surfaces(layout.piece(shelf).unwrap())[1];
        let (sx, sy) = view.screen(surface.at.0, surface.at.1);
        let point = (sx, sy - surface.height as f32);
        let (landing, showing) = arranging
            .landing(&view, &layout, &snapshot, point, Some(shelf))
            .unwrap();
        assert_eq!(
            landing,
            Landing::Spot(Spot::On {
                piece: shelf,
                slot: 1
            })
        );
        assert_eq!(showing, Some(Showing::Itself));
        assert!(arranging.put(&mut state, keeper, &snapshot, landing));
        assert_eq!(state.shown_by(&DisplayId::find(3)), Some(keeper));
    }

    #[test]
    fn a_find_from_another_house_moves_here_rather_than_being_copied() {
        let (snapshot, mut state, view) = setup();
        let keeper = snapshot.household.keeper;
        let neighbour = snapshot.village[1].keeper;
        let mut next_door = crate::starter::home(&snapshot);
        next_door.keeper = neighbour;
        room::show(
            &mut next_door,
            0,
            &DisplayId::find(76),
            Spot::Wall {
                side: WallSide::North,
                at: 1,
            },
        );
        state.set_household(next_door);
        let mut arranging = Arranging {
            carrying: Some(Carry::Thing(DisplayId::find(76))),
            ..Arranging::default()
        };
        let at = view.on_wall(WallSide::West, 3, crate::scene::HANG_HEIGHT);
        let layout = state.household(keeper).unwrap().rooms[0].clone();
        let (landing, _) = arranging
            .landing(&view, &layout, &snapshot, (at.0 as f32, at.1 as f32), None)
            .unwrap();
        assert_eq!(
            landing,
            Landing::Spot(Spot::Wall {
                side: WallSide::West,
                at: 3
            })
        );
        assert!(arranging.put(&mut state, keeper, &snapshot, landing));
        assert_eq!(state.shown_by(&DisplayId::find(76)), Some(keeper));
        state.validate().expect("never in two houses at once");
    }

    #[test]
    fn a_piece_put_away_takes_its_finds_back_to_the_drawer() {
        let (snapshot, mut state, _) = setup();
        let keeper = snapshot.household.keeper;
        let home = state.household_mut(keeper).unwrap();
        let shelf = home.rooms[0]
            .pieces
            .iter()
            .find(|p| p.piece.as_str() == "shelf")
            .unwrap()
            .uid;
        room::show(
            home,
            0,
            &DisplayId::find(3),
            Spot::On {
                piece: shelf,
                slot: 0,
            },
        );
        let mut arranging = Arranging {
            carrying: Some(Carry::Piece {
                uid: shelf,
                turn: 0,
            }),
            ..Arranging::default()
        };
        assert!(arranging.put_away(&mut state, keeper));
        assert_eq!(state.shown_by(&DisplayId::find(3)), None);
        assert!(
            state.household(keeper).unwrap().rooms[0]
                .piece(shelf)
                .is_none()
        );
    }

    #[test]
    fn a_shell_cannot_be_stood_on_the_floor_but_a_toy_sheep_can() {
        let (snapshot, state, view) = setup();
        let layout = state.household(snapshot.household.keeper).unwrap().rooms[0].clone();
        let point = view.tile_centre(6, 6);
        let mut arranging = Arranging {
            carrying: Some(Carry::Thing(DisplayId::find(3))),
            ..Arranging::default()
        };
        let (_, shell) = arranging
            .landing(&view, &layout, &snapshot, point, None)
            .unwrap();
        assert_eq!(shell, None);
        arranging.carrying = Some(Carry::Thing(DisplayId::find(132)));
        let (_, sheep) = arranging
            .landing(&view, &layout, &snapshot, point, None)
            .unwrap();
        assert_eq!(sheep, Some(Showing::Itself));
    }
}
