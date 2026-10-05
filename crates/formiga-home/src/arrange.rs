//! Arrange Mode: picking things up, carrying them about the house, putting them down where they
//! fit, and putting them back in the catalogue. Nothing is ever sold or used up: a piece put away
//! is simply not in the house, and a find put away is back in the drawer. A doorway slides along
//! its wall, and a new room is set beside the others wherever there is space for it.
//!
//! Every change can be undone. What is undone is every home at once, since moving a find from
//! another house into this one changes both.

use crate::catalog::{self, Piece, Template};
use crate::house::{self, Height, House, Wall};
use crate::iso::{View, WALL_HEIGHT};
use crate::room::{self, Footprint, Showing};
use formiga_home_contract::limits::MAX_PLAN_REACH;
use formiga_home_contract::{
    CatalogId, DisplayId, Door, HomeSnapshot, HomeState, HouseholdHome, Liked, PlacedDisplay,
    PlacedPiece, PlanPoint, RoomLayout, Spot, TravelerId, WallSide,
};

/// How many changes can be undone.
const HISTORY: usize = 40;

/// What is being carried.
#[derive(Clone, Debug, PartialEq)]
pub enum Carry {
    /// A piece from the catalogue, at a turn.
    New { piece: &'static Piece, turn: u8 },
    /// A piece already in the house, by its name in the house, lifted out while it is carried.
    Piece { name: u16, turn: u8 },
    /// Something the colony has, from this house, another, or the drawer.
    Thing(DisplayId),
    /// A doorway, by the room whose wall it is in.
    Door { room: u8, door: Door },
}

/// Where a carried thing would go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Landing {
    /// On a room's floor, in the room's own tiles.
    Floor { room: u8, x: u8, y: u8 },
    /// A spot in a room, as the room has it.
    Spot { room: u8, spot: Spot },
    /// A stretch of a room's far wall, for a doorway.
    Wall { room: u8, door: Door },
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

    /// The piece being carried, its turn, and its name in the house if it is already in it.
    pub fn carried_piece(&self, house: &House) -> Option<(&'static Piece, u8, Option<u16>)> {
        match &self.carrying {
            Some(Carry::New { piece, turn }) => Some((piece, *turn, None)),
            Some(Carry::Piece { name, turn }) => {
                let placed = house.piece(*name)?;
                Some((catalog::piece(&placed.piece)?, *turn, Some(*name)))
            }
            _ => None,
        }
    }

    /// Where what is carried would go with the pointer at `point` on the scene, and how it would
    /// show; `None` for the showing when it cannot go there. `over_piece` is the piece under the
    /// pointer, by its name in the house.
    pub fn landing(
        &self,
        view: &View,
        house: &House,
        home: &HouseholdHome,
        snapshot: &HomeSnapshot,
        point: (f32, f32),
        over_piece: Option<u16>,
    ) -> Option<(Landing, Option<Showing>)> {
        if let Some((piece, turn, moving)) = self.carried_piece(house) {
            let (w, d) = piece.size_at(turn);
            let (fx, fy) = view.floor_at(point.0, point.1);
            let index = house.room_of_point((fx, fy))?;
            let room = house.room(index)?;
            // The pointer holds the piece by its middle, and keeps it within the room it is over.
            let x = (fx - f32::from(room.x) - f32::from(w) / 2.0 + 0.5).floor();
            let y = (fy - f32::from(room.y) - f32::from(d) / 2.0 + 0.5).floor();
            let x = x.clamp(0.0, f32::from(room.width.saturating_sub(w))) as u8;
            let y = y.clamp(0.0, f32::from(room.depth.saturating_sub(d))) as u8;
            let layout = home.rooms.get(usize::from(index))?;
            let moving_here = moving
                .and_then(|name| house.local(name))
                .filter(|(from, _)| *from == index)
                .map(|(_, uid)| uid);
            let footprint = Footprint {
                x: room.x + x,
                y: room.y + y,
                w,
                d,
            };
            let fits = room::can_place(layout, piece, x, y, turn, moving_here)
                && (piece.flat || !blocks_a_doorway(house, footprint));
            return Some((
                Landing::Floor { room: index, x, y },
                fits.then_some(Showing::Itself),
            ));
        }
        if let Some(Carry::Door { room, door }) = &self.carrying {
            let wall = wall_cell_at(view, house, point)?;
            let fits = can_move_door(house, home, (*room, *door), wall);
            let landing = Landing::Wall {
                room: wall.room,
                door: Door {
                    side: wall.side,
                    at: wall.at,
                },
            };
            return Some((landing, fits.then_some(Showing::Itself)));
        }
        let Some(Carry::Thing(id)) = &self.carrying else {
            return None;
        };
        let item = snapshot.item(id)?;
        // On a piece's surfaces, the free one nearest the pointer.
        if let Some(name) = over_piece
            && let Some(placed) = house.piece(name)
            && let Some((index, uid)) = house.local(name)
            && let Some(layout) = home.rooms.get(usize::from(index))
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
                return Some((Landing::Spot { room: index, spot }, Some(showing)));
            }
        }
        if let Some((index, spot)) = wall_at(view, house, point) {
            let layout = home.rooms.get(usize::from(index))?;
            return Some((
                Landing::Spot { room: index, spot },
                room::can_show(layout, item, spot),
            ));
        }
        let (tx, ty) = view.tile_at(point.0, point.1)?;
        let (index, x, y) = house.to_room(i32::from(tx), i32::from(ty))?;
        let layout = home.rooms.get(usize::from(index))?;
        let spot = Spot::Floor { x, y };
        let showing = room::can_show(layout, item, spot).filter(|_| {
            !blocks_a_doorway(
                house,
                Footprint {
                    x: tx,
                    y: ty,
                    w: 1,
                    d: 1,
                },
            )
        });
        Some((Landing::Spot { room: index, spot }, showing))
    }

    /// Put what is carried down at `landing`, if it fits. Whether it was put down.
    pub fn put(
        &mut self,
        state: &mut HomeState,
        keeper: TravelerId,
        snapshot: &HomeSnapshot,
        house: &House,
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
            (Carry::New { piece, turn }, Landing::Floor { room, x, y }) => {
                let fits = home.rooms.get(usize::from(room)).is_some_and(|layout| {
                    room::can_place(layout, piece, x, y, *turn, None)
                        && (piece.flat
                            || !blocks_a_doorway(house, at(house, room, piece, x, y, *turn)))
                });
                room::has_room_for_more(home) && fits && {
                    room::add_piece(home, usize::from(room), piece, x, y, *turn);
                    true
                }
            }
            (Carry::Piece { name, turn }, Landing::Floor { room, x, y }) => {
                move_piece(home, house, *name, (room, x, y, *turn))
            }
            (Carry::Thing(id), Landing::Spot { room, spot }) => {
                let fits = snapshot.item(id).is_some_and(|item| {
                    home.rooms
                        .get(usize::from(room))
                        .and_then(|layout| room::can_show(layout, item, spot))
                        .is_some()
                });
                let shown_here = home.shown().any(|shown| shown == id);
                if fits && (shown_here || room::has_room_for_more(home)) {
                    room::show(home, usize::from(room), id, spot);
                    true
                } else {
                    false
                }
            }
            (
                Carry::Door { room, door },
                Landing::Wall {
                    room: to,
                    door: there,
                },
            ) => match house.wall(to, there.side, there.at) {
                Some(wall) if can_move_door(house, home, (*room, *door), wall) => {
                    home.rooms[usize::from(*room)]
                        .doors
                        .retain(|kept| kept != door);
                    home.rooms[usize::from(to)].doors.push(there);
                    true
                }
                _ => false,
            },
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

    /// Put what is carried back in the catalogue or the drawer: out of the house, and nothing
    /// lost. A doorway cannot be put away.
    pub fn put_away(&mut self, state: &mut HomeState, keeper: TravelerId, house: &House) -> bool {
        let Some(carrying) = self.carrying.take() else {
            return false;
        };
        self.dragged = false;
        let before = state.clone();
        let Some(home) = state.household_mut(keeper) else {
            return false;
        };
        let changed = match carrying {
            Carry::New { .. } | Carry::Door { .. } => false,
            Carry::Piece { name, .. } => match house.local(name) {
                Some((index, uid)) => {
                    room::remove_piece(&mut home.rooms[usize::from(index)], uid);
                    true
                }
                None => false,
            },
            Carry::Thing(id) => home.take_down(&id),
        };
        if changed {
            self.remember(&before);
        }
        changed
    }

    /// Change one room's floor or walls.
    pub fn finish(
        &mut self,
        state: &mut HomeState,
        keeper: TravelerId,
        room: u8,
        floor: Option<&str>,
        wall: Option<&str>,
    ) {
        let before = state.clone();
        let Some(layout) = state
            .household_mut(keeper)
            .and_then(|home| home.rooms.get_mut(usize::from(room)))
        else {
            return;
        };
        let mut changed = false;
        if let Some(id) = floor.and_then(CatalogId::parse)
            && layout.floor != id
        {
            layout.floor = id;
            changed = true;
        }
        if let Some(id) = wall.and_then(CatalogId::parse)
            && layout.wall != id
        {
            layout.wall = id;
            changed = true;
        }
        if changed {
            self.remember(&before);
        }
    }

    /// Build a room: the household's home becomes `grown`, one of [`room_places`]. What it
    /// took down, from walls now cut down low.
    pub fn build(
        &mut self,
        state: &mut HomeState,
        keeper: TravelerId,
        grown: &HouseholdHome,
    ) -> Vec<DisplayId> {
        let before = state.clone();
        let Some(home) = state.household_mut(keeper) else {
            return Vec::new();
        };
        let taken_down = home
            .shown()
            .filter(|item| !grown.shown().any(|kept| kept == *item))
            .cloned()
            .collect();
        *home = grown.clone();
        self.remember(&before);
        self.carrying = None;
        taken_down
    }

    /// Take a room away, with everything in it: its pieces back in the catalogue and its finds
    /// back in the drawer. Never the first room, nor one the others are only reached through.
    /// Whether it was taken away.
    pub fn take_away_room(&mut self, state: &mut HomeState, keeper: TravelerId, room: u8) -> bool {
        let Some(home) = state.household(keeper) else {
            return false;
        };
        let Some(without) = without_room(home, room) else {
            return false;
        };
        let before = state.clone();
        state.set_household(without);
        self.remember(&before);
        self.carrying = None;
        true
    }
}

/// The footprint of `piece` set at `(x, y)` in a room, on the house's floor.
fn at(house: &House, room: u8, piece: &Piece, x: u8, y: u8, turn: u8) -> Footprint {
    let (w, d) = piece.size_at(turn);
    let (rx, ry) = house.room(room).map_or((0, 0), |room| (room.x, room.y));
    Footprint {
        x: rx + x,
        y: ry + y,
        w,
        d,
    }
}

/// Move a piece already in the house to `(room, x, y, turn)`, taking what is shown on it with
/// it, and what the household thinks of it. Whether it fitted.
fn move_piece(
    home: &mut HouseholdHome,
    house: &House,
    name: u16,
    (to, x, y, turn): (u8, u8, u8, u8),
) -> bool {
    let Some((from, uid)) = house.local(name) else {
        return false;
    };
    let Some(placed) = home.rooms[usize::from(from)].piece(uid).cloned() else {
        return false;
    };
    let Some(piece) = catalog::piece(&placed.piece) else {
        return false;
    };
    let Some(layout) = home.rooms.get(usize::from(to)) else {
        return false;
    };
    let moving = (from == to).then_some(uid);
    if !room::can_place(layout, piece, x, y, turn, moving)
        || (!piece.flat && blocks_a_doorway(house, at(house, to, piece, x, y, turn)))
    {
        return false;
    }
    if from == to {
        if let Some(placed) = home.rooms[usize::from(to)]
            .pieces
            .iter_mut()
            .find(|placed| placed.uid == uid)
        {
            placed.x = x;
            placed.y = y;
            placed.turn = turn;
        }
        return true;
    }
    // Into another room: out of this one with whatever it shows, and in under a name the new
    // room has free.
    let source = &mut home.rooms[usize::from(from)];
    source.pieces.retain(|placed| placed.uid != uid);
    let mut carried = Vec::new();
    source.displays.retain(|shown| match shown.spot {
        Spot::On { piece, slot } if piece == uid => {
            carried.push((shown.item.clone(), slot));
            false
        }
        _ => true,
    });
    let taken = home.rooms[usize::from(to)].piece(uid).is_some();
    let new_uid = if taken { room::next_uid(home) } else { uid };
    let target = &mut home.rooms[usize::from(to)];
    target.pieces.push(PlacedPiece {
        uid: new_uid,
        piece: placed.piece,
        x,
        y,
        turn,
    });
    for (item, slot) in carried {
        target.displays.push(PlacedDisplay {
            item,
            spot: Spot::On {
                piece: new_uid,
                slot,
            },
        });
    }
    let old = Liked::Piece { room: from, uid };
    for liking in &mut home.likings {
        if liking.thing == old {
            liking.thing = Liked::Piece {
                room: to,
                uid: new_uid,
            };
        }
    }
    true
}

/// The tiles either side of every doorway, which nothing may stand on.
fn doorway_tiles(house: &House) -> Vec<(i32, i32)> {
    house
        .walls
        .iter()
        .filter(|wall| wall.door)
        .flat_map(|wall| {
            let front = (i32::from(wall.x), i32::from(wall.y));
            let behind = wall.behind();
            std::iter::once(front).chain(wall.beyond.map(|_| behind))
        })
        .collect()
}

/// Whether something standing on `footprint` of the house's floor would stand in a doorway.
pub fn blocks_a_doorway(house: &House, footprint: Footprint) -> bool {
    doorway_tiles(house)
        .into_iter()
        .any(|(x, y)| x >= 0 && y >= 0 && footprint.contains(x as u8, y as u8))
}

/// Whether a tile of the house's floor is clear of anything standing on it.
fn clear(house: &House, (x, y): (i32, i32)) -> bool {
    house.room_at(x, y).is_some() && {
        let walkable = house.walkable();
        walkable[(y * i32::from(house.width) + x) as usize]
    }
}

/// Whether the doorway `from` could move to the stretch of wall `to`: one that joins the same
/// two rooms, or opens outside as it did; with its tiles clear and nothing hung there.
fn can_move_door(house: &House, home: &HouseholdHome, from: (u8, Door), to: &Wall) -> bool {
    let Some(was) = house.wall(from.0, from.1.side, from.1.at) else {
        return false;
    };
    if to.door {
        return false;
    }
    match (was.beyond, to.beyond) {
        // A front door goes in any full-height wall with nothing of the house behind it.
        (None, None) if to.height == Height::Full => {}
        (Some(_), Some(_)) if (to.room, to.beyond) == (was.room, was.beyond) => {}
        _ => return false,
    }
    let hung = home.rooms[usize::from(to.room)]
        .displays
        .iter()
        .any(|shown| {
            shown.spot
                == Spot::Wall {
                    side: to.side,
                    at: to.at,
                }
        });
    !hung
        && clear(house, (i32::from(to.x), i32::from(to.y)))
        && (to.beyond.is_none() || clear(house, to.behind()))
}

/// The stretch of full-height wall under a point on the scene, high enough above the skirting to
/// hang something, away from any doorway: the room it is in, and the spot.
pub fn wall_at(view: &View, house: &House, point: (f32, f32)) -> Option<(u8, Spot)> {
    house
        .walls
        .iter()
        .filter(|wall| wall.height == Height::Full && !wall.door)
        .find(|wall| on_face(view, wall, point, 10.0..WALL_HEIGHT as f32 - 2.0))
        .map(|wall| {
            (
                wall.room,
                Spot::Wall {
                    side: wall.side,
                    at: wall.at,
                },
            )
        })
}

/// The stretch of any far wall under a point on the scene, low or full, for a doorway.
pub fn wall_cell_at<'a>(view: &View, house: &'a House, point: (f32, f32)) -> Option<&'a Wall> {
    house.walls.iter().find(|wall| {
        let rise = match wall.height {
            Height::Full => WALL_HEIGHT as f32,
            Height::Low => crate::art::shell::LOW_WALL as f32 + 8.0,
        };
        on_face(view, wall, point, -4.0..rise)
    })
}

/// Whether a point on the scene is on a stretch of wall's face, `up` pixels above its foot.
fn on_face(view: &View, wall: &Wall, point: (f32, f32), up: std::ops::Range<f32>) -> bool {
    let (a, b) = wall.foot();
    let (a, b) = (view.screen(a.0, a.1), view.screen(b.0, b.1));
    let across = (point.0 - a.0) / (b.0 - a.0);
    if !(0.0..1.0).contains(&across) {
        return false;
    }
    let floor_y = a.1 + (b.1 - a.1) * across;
    up.contains(&(floor_y - point.1))
}

/// The household's home in `state`, made if it has none yet, and made sound: see [`settle`].
pub fn ensure_home(state: &mut HomeState, snapshot: &HomeSnapshot) -> HouseholdHome {
    if state.household(snapshot.household.keeper).is_none() {
        state.set_household(crate::starter::home(snapshot));
    }
    let home = state
        .household_mut(snapshot.household.keeper)
        .expect("the household's home was just made");
    settle(home);
    home.clone()
}

/// A home made sound for the house it makes: every room given its place on the plan, every room
/// reached through a doorway, a front door, and nothing left hanging where a wall has been cut
/// down or a doorway made. What it took down.
pub fn settle(home: &mut HouseholdHome) -> Vec<DisplayId> {
    let plan = house::set_out(&home.rooms);
    for (layout, at) in home.rooms.iter_mut().zip(&plan).skip(1) {
        if layout.plan.is_none() {
            let reach = i32::from(MAX_PLAN_REACH);
            layout.plan = Some(PlanPoint {
                x: at.0.clamp(-reach, reach) as i8,
                y: at.1.clamp(-reach, reach) as i8,
            });
        }
    }
    // Every room reached: a doorway through the middle-most clear stretch of wall between a room
    // not yet reached and one that is.
    loop {
        let house = House::of(&home.rooms);
        let reached = house.reachable();
        let Some(wall) = house
            .walls
            .iter()
            .filter(|wall| {
                wall.beyond.is_some_and(|beyond| {
                    reached[usize::from(wall.room)] != reached[usize::from(beyond)]
                }) && !wall.door
                    && clear(&house, (i32::from(wall.x), i32::from(wall.y)))
                    && clear(&house, wall.behind())
            })
            .min_by_key(|wall| middling(&house, wall))
            .copied()
        else {
            break;
        };
        home.rooms[usize::from(wall.room)].doors.push(Door {
            side: wall.side,
            at: wall.at,
        });
    }
    // A front door, in the first room's left-hand wall if it can be.
    let house = House::of(&home.rooms);
    if house.front_door().is_none() {
        let hung = |wall: &Wall| {
            home.rooms[usize::from(wall.room)]
                .displays
                .iter()
                .any(|shown| {
                    shown.spot
                        == Spot::Wall {
                            side: wall.side,
                            at: wall.at,
                        }
                })
        };
        let best = house
            .walls
            .iter()
            .filter(|wall| {
                wall.beyond.is_none()
                    && wall.height == Height::Full
                    && !hung(wall)
                    && clear(&house, (i32::from(wall.x), i32::from(wall.y)))
            })
            .min_by_key(|wall| {
                (
                    wall.room,
                    wall.side != WallSide::West,
                    middling(&house, wall),
                )
            })
            .copied();
        if let Some(wall) = best {
            home.rooms[usize::from(wall.room)].doors.push(Door {
                side: wall.side,
                at: wall.at,
            });
        }
    }
    // Nothing hangs on a wall cut down, or in a doorway.
    let house = House::of(&home.rooms);
    let mut taken_down = Vec::new();
    for (index, layout) in home.rooms.iter_mut().enumerate() {
        layout.displays.retain(|shown| {
            let Spot::Wall { side, at } = shown.spot else {
                return true;
            };
            let stands = house
                .wall(index as u8, side, at)
                .is_some_and(|wall| wall.height == Height::Full && !wall.door);
            if !stands {
                taken_down.push(shown.item.clone());
            }
            stands
        });
    }
    taken_down
}

/// How far a stretch of wall is from the middle of its run along its room, for choosing the
/// middle-most.
fn middling(house: &House, wall: &Wall) -> u32 {
    let length = house
        .room(wall.room)
        .map_or(1, |room| room.wall_length(wall.side));
    (i32::from(wall.at) * 2 + 1 - i32::from(length)).unsigned_abs()
}

/// The home without one of its rooms, if it can do without it: never the first, and never one
/// the others are only reached through. Doorways that opened onto it close, and whatever liked
/// what was in it forgets.
fn without_room(home: &HouseholdHome, room: u8) -> Option<HouseholdHome> {
    if room == 0 || usize::from(room) >= home.rooms.len() {
        return None;
    }
    let house = House::of(&home.rooms);
    let mut kept = home.clone();
    for wall in &house.walls {
        if wall.door && wall.beyond == Some(room) {
            kept.rooms[usize::from(wall.room)]
                .doors
                .retain(|door| (door.side, door.at) != (wall.side, wall.at));
        }
    }
    kept.rooms.remove(usize::from(room));
    kept.likings.retain_mut(|liking| match &mut liking.thing {
        Liked::Piece { room: r, .. } if *r == room => false,
        Liked::Piece { room: r, .. } => {
            if *r > room {
                *r -= 1;
            }
            true
        }
        Liked::Shown { .. } => true,
    });
    let reached = House::of(&kept.rooms).reachable();
    if reached.iter().any(|reached| !reached) {
        return None;
    }
    settle(&mut kept);
    Some(kept)
}

/// Whether a room could be taken away.
pub fn can_take_away(home: &HouseholdHome, room: u8) -> bool {
    without_room(home, room).is_some()
}

/// Every place a room of kind `template` could be built: beside one of the rooms already there,
/// middle to middle, and joined to it by a doorway. Each is the home it would make, behind the
/// house first and then in front of it. The room comes with those of its kind's pieces that have
/// arrived for the colony `snapshot` describes.
pub fn room_places(
    home: &HouseholdHome,
    template: &'static Template,
    snapshot: &HomeSnapshot,
) -> Vec<HouseholdHome> {
    let plan = house::set_out(&home.rooms);
    let (nw, nd) = (i32::from(template.size.0), i32::from(template.size.1));
    let mut places = Vec::new();
    let mut seen = Vec::new();
    for side in [Side::North, Side::West, Side::South, Side::East] {
        for (index, layout) in home.rooms.iter().enumerate() {
            let (px, py) = plan[index];
            let (w, d) = (i32::from(layout.width), i32::from(layout.depth));
            let at = match side {
                Side::North => (px + (w - nw) / 2, py - nd),
                Side::West => (px - nw, py + (d - nd) / 2),
                Side::South => (px + (w - nw) / 2, py + d),
                Side::East => (px + w, py + (d - nd) / 2),
            };
            if seen.contains(&at) {
                continue;
            }
            let reach = i32::from(MAX_PLAN_REACH);
            let inside =
                at.0 >= -reach && at.1 >= -reach && at.0 + nw <= reach && at.1 + nd <= reach;
            let overlaps = home.rooms.iter().zip(&plan).any(|(other, &(ox, oy))| {
                at.0 < ox + i32::from(other.width)
                    && ox < at.0 + nw
                    && at.1 < oy + i32::from(other.depth)
                    && oy < at.1 + nd
            });
            if !inside || overlaps {
                continue;
            }
            let arrived =
                |piece: &Piece| piece.available(snapshot.days_lived, snapshot.inventory.len());
            if let Some(grown) = grow(
                home,
                template,
                (at.0 as i8, at.1 as i8),
                index as u8,
                &arrived,
            ) {
                seen.push(at);
                places.push(grown);
            }
        }
    }
    places
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    North,
    West,
    South,
    East,
}

/// The home with a room of kind `template` set at `at` on the plan, joined to the room
/// `beside` by a doorway in their shared wall where its tiles are clear, nearest the middle.
fn grow(
    home: &HouseholdHome,
    template: &'static Template,
    at: (i8, i8),
    beside: u8,
    arrived: &dyn Fn(&Piece) -> bool,
) -> Option<HouseholdHome> {
    let mut grown = home.clone();
    let mut uid = room::next_uid(home);
    let first = &home.rooms[0];
    let mut room = RoomLayout {
        width: template.size.0,
        depth: template.size.1,
        floor: first.floor.clone(),
        wall: first.wall.clone(),
        pieces: Vec::new(),
        displays: Vec::new(),
        plan: Some(PlanPoint { x: at.0, y: at.1 }),
        kind: Some(CatalogId::known(template.id)),
        doors: Vec::new(),
    };
    for &(id, x, y, turn) in template.pieces {
        let Some(piece) = catalog::PIECES
            .iter()
            .find(|piece| piece.id == id && arrived(piece))
        else {
            continue;
        };
        room.pieces.push(PlacedPiece {
            uid,
            piece: piece.catalog_id(),
            x,
            y,
            turn,
        });
        uid += 1;
    }
    grown.rooms.push(room);
    let new = (grown.rooms.len() - 1) as u8;
    let house = House::of(&grown.rooms);
    let joins = |wall: &Wall| {
        (wall.room, wall.beyond) == (new, Some(beside))
            || (wall.room, wall.beyond) == (beside, Some(new))
    };
    // A front door in the wall the new room now shares is the way through already. Whatever
    // hangs on that wall comes down, since the wall is cut down low.
    if !house.walls.iter().any(|wall| joins(wall) && wall.door) {
        let wall = house
            .walls
            .iter()
            .filter(|wall| {
                joins(wall)
                    && clear(&house, (i32::from(wall.x), i32::from(wall.y)))
                    && clear(&house, wall.behind())
            })
            .min_by_key(|wall| shared_middling(&house, wall, new, beside))
            .copied()?;
        grown.rooms[usize::from(wall.room)].doors.push(Door {
            side: wall.side,
            at: wall.at,
        });
    }
    settle(&mut grown);
    Some(grown)
}

/// How far a stretch of the wall two rooms share is from the middle of what they share.
fn shared_middling(house: &House, wall: &Wall, a: u8, b: u8) -> u32 {
    let shared: Vec<u8> = house
        .walls
        .iter()
        .filter(|other| {
            other.side == wall.side
                && ((other.room, other.beyond) == (a, Some(b))
                    || (other.room, other.beyond) == (b, Some(a)))
        })
        .map(|other| other.at)
        .collect();
    let low = shared.iter().min().copied().unwrap_or(wall.at);
    let high = shared.iter().max().copied().unwrap_or(wall.at);
    (i32::from(wall.at) * 2 - i32::from(low) - i32::from(high)).unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::{HomeDocument, sample};

    fn setup() -> (HomeSnapshot, HomeState, House, View) {
        let snapshot = sample::snapshot();
        let mut state = HomeState::new(&snapshot.colony_key);
        let home = ensure_home(&mut state, &snapshot);
        let house = House::of(&home.rooms);
        let view = View::of(&house);
        (snapshot, state, house, view)
    }

    fn home(state: &HomeState, snapshot: &HomeSnapshot) -> HouseholdHome {
        state.household(snapshot.household.keeper).unwrap().clone()
    }

    #[test]
    fn a_new_piece_goes_where_it_fits_and_can_be_undone_and_redone() {
        let (snapshot, mut state, house, view) = setup();
        let keeper = snapshot.household.keeper;
        let mut arranging = Arranging::default();
        let lamp = catalog::PIECES.iter().find(|p| p.id == "lamp").unwrap();
        arranging.carrying = Some(Carry::New {
            piece: lamp,
            turn: 0,
        });
        let before_pieces = home(&state, &snapshot).rooms[0].pieces.len();
        let point = view.tile_centre(6, 6);
        let (landing, showing) = arranging
            .landing(
                &view,
                &house,
                &home(&state, &snapshot),
                &snapshot,
                point,
                None,
            )
            .unwrap();
        assert_eq!(
            landing,
            Landing::Floor {
                room: 0,
                x: 6,
                y: 6
            }
        );
        assert_eq!(showing, Some(Showing::Itself));
        let before = state.clone();
        assert!(arranging.put(&mut state, keeper, &snapshot, &house, landing));
        assert_eq!(
            home(&state, &snapshot).rooms[0].pieces.len(),
            before_pieces + 1
        );
        assert!(arranging.undo(&mut state));
        assert_eq!(state, before);
        assert!(arranging.redo(&mut state));
        state.validate().unwrap();
    }

    #[test]
    fn nothing_stands_in_the_front_doorway() {
        let (snapshot, state, house, view) = setup();
        let door = house.front_door().expect("every house has a way in");
        let arranging = Arranging {
            carrying: Some(Carry::New {
                piece: catalog::PIECES.iter().find(|p| p.id == "cushion").unwrap(),
                turn: 0,
            }),
            ..Arranging::default()
        };
        let point = view.tile_centre(door.x, door.y);
        let (_, showing) = arranging
            .landing(
                &view,
                &house,
                &home(&state, &snapshot),
                &snapshot,
                point,
                None,
            )
            .unwrap();
        assert_eq!(showing, None);
    }

    #[test]
    fn a_find_dragged_onto_a_shelf_lands_on_the_shelf() {
        let (snapshot, mut state, house, view) = setup();
        let keeper = snapshot.household.keeper;
        let layout = home(&state, &snapshot).rooms[0].clone();
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
        let name = house.named(0, shelf).unwrap();
        let surface = room::surfaces(house.piece(name).unwrap())[1];
        let (sx, sy) = view.screen(surface.at.0, surface.at.1);
        let point = (sx, sy - surface.height as f32);
        let (landing, showing) = arranging
            .landing(
                &view,
                &house,
                &home(&state, &snapshot),
                &snapshot,
                point,
                Some(name),
            )
            .unwrap();
        assert_eq!(
            landing,
            Landing::Spot {
                room: 0,
                spot: Spot::On {
                    piece: shelf,
                    slot: 1
                }
            }
        );
        assert_eq!(showing, Some(Showing::Itself));
        assert!(arranging.put(&mut state, keeper, &snapshot, &house, landing));
        assert_eq!(state.shown_by(&DisplayId::find(3)), Some(keeper));
    }

    #[test]
    fn a_find_from_another_house_moves_here_rather_than_being_copied() {
        let (snapshot, mut state, house, view) = setup();
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
        let wall = house.wall(0, WallSide::West, 5).unwrap();
        let at = view.on_wall(wall, crate::scene::HANG_HEIGHT);
        let (landing, _) = arranging
            .landing(
                &view,
                &house,
                &home(&state, &snapshot),
                &snapshot,
                (at.0 as f32, at.1 as f32),
                None,
            )
            .unwrap();
        assert_eq!(
            landing,
            Landing::Spot {
                room: 0,
                spot: Spot::Wall {
                    side: WallSide::West,
                    at: 5
                }
            }
        );
        assert!(arranging.put(&mut state, keeper, &snapshot, &house, landing));
        assert_eq!(state.shown_by(&DisplayId::find(76)), Some(keeper));
        state.validate().expect("never in two houses at once");
    }

    #[test]
    fn a_piece_put_away_takes_its_finds_back_to_the_drawer() {
        let (snapshot, mut state, house, _) = setup();
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
                name: house.named(0, shelf).unwrap(),
                turn: 0,
            }),
            ..Arranging::default()
        };
        assert!(arranging.put_away(&mut state, keeper, &house));
        assert_eq!(state.shown_by(&DisplayId::find(3)), None);
        assert!(
            state.household(keeper).unwrap().rooms[0]
                .piece(shelf)
                .is_none()
        );
    }

    #[test]
    fn a_shell_cannot_be_stood_on_the_floor_but_a_toy_sheep_can() {
        let (snapshot, state, house, view) = setup();
        let point = view.tile_centre(6, 6);
        let mut arranging = Arranging {
            carrying: Some(Carry::Thing(DisplayId::find(3))),
            ..Arranging::default()
        };
        let here = home(&state, &snapshot);
        let (_, shell) = arranging
            .landing(&view, &house, &here, &snapshot, point, None)
            .unwrap();
        assert_eq!(shell, None);
        arranging.carrying = Some(Carry::Thing(DisplayId::find(132)));
        let (_, sheep) = arranging
            .landing(&view, &house, &here, &snapshot, point, None)
            .unwrap();
        assert_eq!(sheep, Some(Showing::Itself));
    }

    #[test]
    fn a_new_room_goes_beside_the_house_joined_by_a_doorway_and_keeps_the_house_sound() {
        let (snapshot, mut state, _, _) = setup();
        let keeper = snapshot.household.keeper;
        let nook = catalog::ROOMS.iter().find(|t| t.id == "room.nook").unwrap();
        let places = room_places(&home(&state, &snapshot), nook, &snapshot);
        assert!(places.len() >= 3, "{}", places.len());
        for grown in &places {
            let house = House::of(&grown.rooms);
            assert_eq!(house.reachable(), vec![true, true]);
            assert!(house.front_door().is_some());
            let mut whole = state.clone();
            whole.set_household(grown.clone());
            whole.validate().unwrap();
        }
        let mut arranging = Arranging::default();
        arranging.build(&mut state, keeper, &places[0]);
        assert_eq!(home(&state, &snapshot).rooms.len(), 2);
        // A third room can go beside either.
        let gallery = catalog::ROOMS
            .iter()
            .find(|t| t.id == "room.gallery")
            .unwrap();
        let more = room_places(&home(&state, &snapshot), gallery, &snapshot);
        assert!(!more.is_empty());
        arranging.build(&mut state, keeper, &more[0]);
        let three = home(&state, &snapshot);
        assert_eq!(House::of(&three.rooms).reachable(), vec![true; 3]);
        // The first room stays; the last can go, and takes its pieces with it.
        assert!(!can_take_away(&three, 0));
        assert!(arranging.take_away_room(&mut state, keeper, 2));
        assert_eq!(home(&state, &snapshot).rooms.len(), 2);
        assert!(arranging.undo(&mut state));
        assert_eq!(home(&state, &snapshot).rooms.len(), 3);
    }

    #[test]
    fn a_doorway_slides_along_its_wall_but_never_onto_a_picture() {
        let (snapshot, mut state, _, _) = setup();
        let keeper = snapshot.household.keeper;
        // Nothing against the far wall the nook will share, so the doorway has room to slide.
        state.household_mut(keeper).unwrap().rooms[0]
            .pieces
            .retain(|placed| placed.y > 0);
        let nook = catalog::ROOMS.iter().find(|t| t.id == "room.nook").unwrap();
        let grown = room_places(&home(&state, &snapshot), nook, &snapshot).remove(0);
        let mut arranging = Arranging::default();
        arranging.build(&mut state, keeper, &grown);
        let here = home(&state, &snapshot);
        let house = House::of(&here.rooms);
        let view = View::of(&house);
        let door = *house
            .walls
            .iter()
            .find(|wall| wall.door && wall.beyond.is_some())
            .unwrap();
        let along = house
            .walls
            .iter()
            .find(|wall| {
                (wall.room, wall.beyond, wall.side) == (door.room, door.beyond, door.side)
                    && !wall.door
                    && clear(&house, (i32::from(wall.x), i32::from(wall.y)))
                    && clear(&house, wall.behind())
            })
            .copied()
            .expect("somewhere else along the shared wall");
        arranging.carrying = Some(Carry::Door {
            room: door.room,
            door: Door {
                side: door.side,
                at: door.at,
            },
        });
        let (mx, my) = along.middle();
        let (sx, sy) = view.screen(mx, my);
        let (landing, fits) = arranging
            .landing(&view, &house, &here, &snapshot, (sx, sy - 4.0), None)
            .unwrap();
        assert_eq!(fits, Some(Showing::Itself));
        assert!(arranging.put(&mut state, keeper, &snapshot, &house, landing));
        let moved = House::of(&home(&state, &snapshot).rooms);
        assert!(moved.wall(along.room, along.side, along.at).unwrap().door);
        assert_eq!(moved.reachable(), vec![true, true]);
        // The front door cannot go where a picture hangs.
        let front = *moved.front_door().unwrap();
        let picture = moved
            .walls
            .iter()
            .find(|wall| wall.beyond.is_none() && wall.height == Height::Full && !wall.door)
            .copied()
            .unwrap();
        let mut hung = home(&state, &snapshot);
        room::show(
            &mut hung,
            usize::from(picture.room),
            &DisplayId::find(76),
            Spot::Wall {
                side: picture.side,
                at: picture.at,
            },
        );
        assert!(!can_move_door(
            &moved,
            &hung,
            (
                front.room,
                Door {
                    side: front.side,
                    at: front.at
                }
            ),
            &picture
        ));
    }
}
