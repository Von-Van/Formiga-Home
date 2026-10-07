//! The house as one floor. Each room keeps its own layout, in its own tiles, as the state keeps
//! it; here every room is set out where it stands on the house's plan, so the residents walk from
//! room to room through the doorways and the whole house is drawn as one cutaway.
//!
//! Positions here are tiles on the house's floor: the plan moved so that its far corner is at
//! 0, 0. Every piece has a name of its own across the house, so whatever a resident is doing can
//! name a piece without naming its room. The house is made again whenever a layout changes.
//!
//! A cutaway shows every room only if no wall stands in front of one. A wall between two rooms is
//! cut down low, and so is any far wall that would hide part of another room behind it. Only a
//! wall with nothing of the house behind it stands at full height, and only there can something
//! be hung.

use crate::catalog;
use crate::iso::{TILE_W, WALL_HEIGHT};
use crate::placement::{self, Footprint, Place};
use formiga_home_contract::{CatalogId, DisplayId, Liked, PlacedPiece, RoomLayout, Spot, WallSide};
use std::collections::BTreeSet;

/// A room where it stands on the house's floor.
#[derive(Clone, Debug, PartialEq)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub depth: u8,
    pub floor: CatalogId,
    pub wall: CatalogId,
    pub kind: Option<CatalogId>,
}

impl Room {
    /// How long one of its far walls is.
    pub fn wall_length(&self, side: WallSide) -> u8 {
        match side {
            WallSide::North => self.width,
            WallSide::West => self.depth,
        }
    }
}

/// How tall a stretch of far wall stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Height {
    Full,
    /// Cut down, so that what is behind it shows.
    Low,
}

/// A tile's length of one room's far wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wall {
    pub room: u8,
    pub side: WallSide,
    pub at: u8,
    /// The tile in front of it, on the house's floor.
    pub x: u8,
    pub y: u8,
    pub height: Height,
    pub door: bool,
    /// The room on its other side, if it is not outside.
    pub beyond: Option<u8>,
}

impl Wall {
    /// The tile on its other side, which may be outside the house.
    pub fn behind(&self) -> (i32, i32) {
        let (x, y) = (i32::from(self.x), i32::from(self.y));
        match self.side {
            WallSide::North => (x, y - 1),
            WallSide::West => (x - 1, y),
        }
    }

    /// The two ends of its foot on the house's floor, the one nearer its room's far corner first.
    pub fn foot(&self) -> ((f32, f32), (f32, f32)) {
        let (x, y) = (f32::from(self.x), f32::from(self.y));
        match self.side {
            WallSide::North => ((x, y), (x + 1.0, y)),
            WallSide::West => ((x, y), (x, y + 1.0)),
        }
    }

    /// The middle of its foot.
    pub fn middle(&self) -> (f32, f32) {
        let (a, b) = self.foot();
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    }

    /// A doorway out of the house: the front door.
    pub fn opens_outside(&self) -> bool {
        self.door && self.beyond.is_none()
    }
}

/// Where something is shown, anywhere in the house.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum At {
    /// On a piece's surface, by the piece's name in the house.
    On { piece: u16, slot: u8 },
    /// Hung on a room's far wall, as the room has it.
    Wall { room: u8, side: WallSide, at: u8 },
    /// Standing on a tile of the house's floor.
    Floor { x: u8, y: u8 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub item: DisplayId,
    pub at: At,
}

#[derive(Clone, Debug, PartialEq)]
pub struct House {
    pub rooms: Vec<Room>,
    pub width: u8,
    pub depth: u8,
    /// Which room each tile of the house's floor is in, row by row.
    cells: Vec<Option<u8>>,
    pub walls: Vec<Wall>,
    /// Every piece in the house, on the house's floor and under its name in the house.
    pub pieces: Vec<PlacedPiece>,
    pub shown: Vec<Shown>,
    /// Each piece's name in the house, its room, and its name in its room.
    names: Vec<(u16, u8, u16)>,
}

/// How far behind a full-height wall its top reaches on screen, in tiles along both floor axes.
const WALL_REACH: f32 = WALL_HEIGHT as f32 / TILE_W as f32;

impl House {
    /// The house the rooms make.
    pub fn of(layouts: &[RoomLayout]) -> Self {
        let plan = set_out(layouts);
        let left = plan.iter().map(|at| at.0).min().unwrap_or(0);
        let top = plan.iter().map(|at| at.1).min().unwrap_or(0);
        let rooms: Vec<Room> = layouts
            .iter()
            .zip(&plan)
            .map(|(layout, at)| Room {
                x: (at.0 - left) as u8,
                y: (at.1 - top) as u8,
                width: layout.width,
                depth: layout.depth,
                floor: layout.floor.clone(),
                wall: layout.wall.clone(),
                kind: layout.kind.clone(),
            })
            .collect();
        let width = rooms
            .iter()
            .map(|room| room.x + room.width)
            .max()
            .unwrap_or(1);
        let depth = rooms
            .iter()
            .map(|room| room.y + room.depth)
            .max()
            .unwrap_or(1);
        let mut cells = vec![None; usize::from(width) * usize::from(depth)];
        for (index, room) in rooms.iter().enumerate() {
            for y in room.y..room.y + room.depth {
                for x in room.x..room.x + room.width {
                    cells[usize::from(y) * usize::from(width) + usize::from(x)] = Some(index as u8);
                }
            }
        }
        let mut house = Self {
            rooms,
            width,
            depth,
            cells,
            walls: Vec::new(),
            pieces: Vec::new(),
            shown: Vec::new(),
            names: name_pieces(layouts),
        };
        house.walls = house.raise_walls(layouts);
        for (index, layout) in layouts.iter().enumerate() {
            let room = &house.rooms[index];
            let (rx, ry) = (room.x, room.y);
            for placed in &layout.pieces {
                let Some(name) = house.named(index as u8, placed.uid) else {
                    continue;
                };
                house.pieces.push(PlacedPiece {
                    uid: name,
                    piece: placed.piece.clone(),
                    x: rx + placed.x,
                    y: ry + placed.y,
                    turn: placed.turn,
                });
            }
            for shown in &layout.displays {
                let at = match shown.spot {
                    Spot::On { piece, slot } => match house.named(index as u8, piece) {
                        Some(piece) => At::On { piece, slot },
                        None => continue,
                    },
                    Spot::Wall { side, at } => At::Wall {
                        room: index as u8,
                        side,
                        at,
                    },
                    Spot::Floor { x, y } => At::Floor {
                        x: rx + x,
                        y: ry + y,
                    },
                };
                house.shown.push(Shown {
                    item: shown.item.clone(),
                    at,
                });
            }
        }
        house
    }

    /// Every tile's length of every room's far walls, each as tall as it can stand without
    /// hiding another room.
    fn raise_walls(&self, layouts: &[RoomLayout]) -> Vec<Wall> {
        let mut walls = Vec::new();
        for (index, (room, layout)) in self.rooms.iter().zip(layouts).enumerate() {
            for side in [WallSide::North, WallSide::West] {
                for at in 0..room.wall_length(side) {
                    let (x, y) = match side {
                        WallSide::North => (room.x + at, room.y),
                        WallSide::West => (room.x, room.y + at),
                    };
                    let mut wall = Wall {
                        room: index as u8,
                        side,
                        at,
                        x,
                        y,
                        height: Height::Full,
                        door: layout.has_door(side, at),
                        beyond: None,
                    };
                    let (bx, by) = wall.behind();
                    wall.beyond = self.room_at(bx, by);
                    if wall.beyond.is_some() || self.would_hide(&wall) {
                        wall.height = Height::Low;
                    }
                    walls.push(wall);
                }
            }
        }
        walls
    }

    /// Whether a full-height wall here would stand in front of some of another room's floor.
    fn would_hide(&self, wall: &Wall) -> bool {
        let (line, along) = match wall.side {
            WallSide::North => (f32::from(wall.y), f32::from(wall.x)),
            WallSide::West => (f32::from(wall.x), f32::from(wall.y)),
        };
        (0..self.depth).any(|ty| {
            (0..self.width).any(|tx| {
                let Some(room) = self.room_at(i32::from(tx), i32::from(ty)) else {
                    return false;
                };
                if room == wall.room {
                    return false;
                }
                let (fx, fy) = (f32::from(tx) + 0.5, f32::from(ty) + 0.5);
                // How far behind the wall's line, and where along it, the tile is seen.
                let (behind, across) = match wall.side {
                    WallSide::North => (line - fy, fx - fy + line),
                    WallSide::West => (line - fx, fy - fx + line),
                };
                behind > 0.0
                    && behind < WALL_REACH + 0.5
                    && across > along - 1.0
                    && across < along + 2.0
            })
        })
    }

    pub fn room_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.depth) {
            return None;
        }
        self.cells[(y * i32::from(self.width) + x) as usize]
    }

    /// The room a point on the house's floor is in.
    pub fn room_of_point(&self, point: (f32, f32)) -> Option<u8> {
        self.room_at(point.0.floor() as i32, point.1.floor() as i32)
    }

    pub fn room(&self, index: u8) -> Option<&Room> {
        self.rooms.get(usize::from(index))
    }

    pub fn piece(&self, name: u16) -> Option<&PlacedPiece> {
        self.pieces.iter().find(|piece| piece.uid == name)
    }

    /// A piece's room, and its name there.
    pub fn local(&self, name: u16) -> Option<(u8, u16)> {
        self.names
            .iter()
            .find(|(house, ..)| *house == name)
            .map(|&(_, room, uid)| (room, uid))
    }

    /// The house's name for the piece a room calls `uid`.
    pub fn named(&self, room: u8, uid: u16) -> Option<u16> {
        self.names
            .iter()
            .find(|&&(_, r, u)| r == room && u == uid)
            .map(|&(house, ..)| house)
    }

    /// A piece, as a household's likings name it.
    pub fn liked(&self, name: u16) -> Option<Liked> {
        let (room, uid) = self.local(name)?;
        Some(Liked::Piece { room, uid })
    }

    /// The piece a liking is for, by its name in the house.
    pub fn of_liked(&self, liked: &Liked) -> Option<u16> {
        match liked {
            Liked::Piece { room, uid } => self.named(*room, *uid),
            Liked::Shown { .. } => None,
        }
    }

    /// The stretch of a room's far wall `at` tiles along.
    pub fn wall(&self, room: u8, side: WallSide, at: u8) -> Option<&Wall> {
        self.walls
            .iter()
            .find(|wall| wall.room == room && wall.side == side && wall.at == at)
    }

    /// The way in from outside, where visitors knock: the first front door.
    pub fn front_door(&self) -> Option<&Wall> {
        self.walls.iter().find(|wall| wall.opens_outside())
    }

    /// Whether someone can step straight from one tile to the next: within a room, or through a
    /// doorway between two. The way-finding has its own copy of this, kept for every step.
    #[cfg(test)]
    pub fn passable(&self, from: (i32, i32), to: (i32, i32)) -> bool {
        let (Some(a), Some(b)) = (self.room_at(from.0, from.1), self.room_at(to.0, to.1)) else {
            return false;
        };
        if a == b {
            return true;
        }
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        if dx.abs() + dy.abs() != 1 {
            return false;
        }
        // The doorway belongs to whichever of the two is in front.
        let (front, back) = if dx + dy < 0 { (from, to) } else { (to, from) };
        self.walls.iter().any(|wall| {
            wall.door && (i32::from(wall.x), i32::from(wall.y)) == front && wall.behind() == back
        })
    }

    /// The rooms someone can walk to from the first, through the doorways.
    pub fn reachable(&self) -> Vec<bool> {
        let mut reached = vec![false; self.rooms.len()];
        if reached.is_empty() {
            return reached;
        }
        reached[0] = true;
        loop {
            let mut grew = false;
            for wall in self.walls.iter().filter(|wall| wall.door) {
                let Some(beyond) = wall.beyond else { continue };
                let (a, b) = (usize::from(wall.room), usize::from(beyond));
                if reached[a] != reached[b] {
                    reached[a] = true;
                    reached[b] = true;
                    grew = true;
                }
            }
            if !grew {
                return reached;
            }
        }
    }

    /// The kind of place a spot is, if the house has it: a full-height wall's stretch only, away
    /// from its doorway.
    pub fn place_of(&self, at: At) -> Option<Place> {
        match at {
            At::On { piece, slot } => {
                let surface = placement::surfaces(self.piece(piece)?)
                    .into_iter()
                    .nth(usize::from(slot))?;
                Some(match surface.holds {
                    catalog::Holds::Top => Place::Top,
                    catalog::Holds::Shelf => Place::Shelf,
                })
            }
            At::Wall { room, side, at } => self
                .wall(room, side, at)
                .filter(|wall| wall.height == Height::Full && !wall.door)
                .map(|_| Place::Wall),
            At::Floor { x, y } => self
                .room_at(i32::from(x), i32::from(y))
                .map(|_| Place::Floor),
        }
    }

    /// Every tile of the house's floor, row by row, `true` where someone can stand.
    pub fn walkable(&self) -> Vec<bool> {
        let mut grid: Vec<bool> = self.cells.iter().map(Option::is_some).collect();
        let width = usize::from(self.width);
        for placed in &self.pieces {
            if catalog::piece(&placed.piece).is_some_and(|piece| piece.flat) {
                continue;
            }
            for (x, y) in placement::footprint(placed).tiles() {
                if x < self.width && y < self.depth {
                    grid[usize::from(y) * width + usize::from(x)] = false;
                }
            }
        }
        for shown in &self.shown {
            if let At::Floor { x, y } = shown.at
                && x < self.width
                && y < self.depth
            {
                grid[usize::from(y) * width + usize::from(x)] = false;
            }
        }
        grid
    }

    /// The tiles in front of the house's near edges, where the floor is cut away and shows its
    /// thickness: along each room's front on the left (`true`) and on the right (`false`).
    pub fn cut_edges(&self) -> Vec<(u8, u8, bool)> {
        let mut edges = Vec::new();
        for room in &self.rooms {
            let front = room.y + room.depth;
            for x in room.x..room.x + room.width {
                if self.room_at(i32::from(x), i32::from(front)).is_none() {
                    edges.push((x, front - 1, true));
                }
            }
            let side = room.x + room.width;
            for y in room.y..room.y + room.depth {
                if self.room_at(i32::from(side), i32::from(y)).is_none() {
                    edges.push((side - 1, y, false));
                }
            }
        }
        edges
    }

    /// The footprint a room covers on the house's floor.
    pub fn footprint(&self, index: u8) -> Option<Footprint> {
        let room = self.room(index)?;
        Some(Footprint {
            x: room.x,
            y: room.y,
            w: room.width,
            d: room.depth,
        })
    }

    /// A tile of the house's floor, in the room it is in: the room, and the tile there.
    pub fn to_room(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        let index = self.room_at(x, y)?;
        let room = self.room(index)?;
        Some((
            index,
            (x - i32::from(room.x)) as u8,
            (y - i32::from(room.y)) as u8,
        ))
    }

    /// Where something a room shows at `spot` is, in the house.
    pub fn at(&self, room: u8, spot: Spot) -> Option<At> {
        let at = self.room(room)?;
        Some(match spot {
            Spot::On { piece, slot } => At::On {
                piece: self.named(room, piece)?,
                slot,
            },
            Spot::Wall { side, at } => At::Wall { room, side, at },
            Spot::Floor { x, y } => At::Floor {
                x: at.x + x,
                y: at.y + y,
            },
        })
    }

    /// The same, back in the room's own terms.
    pub fn spot(&self, at: At) -> Option<(u8, Spot)> {
        Some(match at {
            At::On { piece, slot } => {
                let (room, uid) = self.local(piece)?;
                (room, Spot::On { piece: uid, slot })
            }
            At::Wall { room, side, at } => (room, Spot::Wall { side, at }),
            At::Floor { x, y } => {
                let (room, x, y) = self.to_room(i32::from(x), i32::from(y))?;
                (room, Spot::Floor { x, y })
            }
        })
    }
}

/// Where each room stands on the plan: where its layout says, or, for a room given no place, past
/// the east end of every other.
pub fn set_out(layouts: &[RoomLayout]) -> Vec<(i32, i32)> {
    let east = layouts
        .iter()
        .enumerate()
        .map(|(index, layout)| match (index, layout.plan) {
            (0, _) => i32::from(layout.width),
            (_, Some(at)) => i32::from(at.x) + i32::from(layout.width),
            (_, None) => 0,
        })
        .max()
        .unwrap_or(0);
    let mut next = east;
    layouts
        .iter()
        .enumerate()
        .map(|(index, layout)| match (index, layout.plan) {
            (0, _) => (0, 0),
            (_, Some(at)) => (i32::from(at.x), i32::from(at.y)),
            (_, None) => {
                let at = (next, 0);
                next += i32::from(layout.width);
                at
            }
        })
        .collect()
}

/// A name in the house for every piece: its own name in its room, unless a piece in an earlier
/// room already has that name, in which case the next one free.
fn name_pieces(layouts: &[RoomLayout]) -> Vec<(u16, u8, u16)> {
    let mut used = BTreeSet::new();
    let mut names = Vec::new();
    let mut waiting = Vec::new();
    for (room, layout) in layouts.iter().enumerate() {
        for placed in &layout.pieces {
            if used.insert(placed.uid) {
                names.push((placed.uid, room as u8, placed.uid));
            } else {
                waiting.push((room as u8, placed.uid));
            }
        }
    }
    let mut free = used.iter().max().map_or(1, |uid| uid.saturating_add(1));
    for (room, uid) in waiting {
        while used.contains(&free) {
            free = free.wrapping_add(1);
        }
        used.insert(free);
        names.push((free, room, uid));
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::{Door, PlanPoint, sample};

    fn front_room() -> RoomLayout {
        crate::starter::room(&sample::snapshot())
    }

    fn nook(x: i8, y: i8) -> RoomLayout {
        RoomLayout {
            width: 4,
            depth: 4,
            floor: CatalogId::known("floor.rose"),
            wall: CatalogId::known("wall.stripes"),
            pieces: vec![PlacedPiece {
                uid: 1,
                piece: CatalogId::known("cushion"),
                x: 1,
                y: 1,
                turn: 0,
            }],
            displays: Vec::new(),
            plan: Some(PlanPoint { x, y }),
            kind: Some(CatalogId::known("room.nook")),
            doors: Vec::new(),
        }
    }

    #[test]
    fn one_room_is_its_own_house() {
        let layout = front_room();
        let house = House::of(std::slice::from_ref(&layout));
        assert_eq!((house.width, house.depth), (layout.width, layout.depth));
        assert_eq!(house.pieces, layout.pieces);
        assert!(
            house
                .walls
                .iter()
                .all(|wall| wall.height == Height::Full && wall.beyond.is_none())
        );
        assert_eq!(house.walls.len(), usize::from(layout.width + layout.depth));
    }

    #[test]
    fn a_room_behind_another_cuts_their_wall_down_and_a_doorway_joins_them() {
        let mut front = front_room();
        front.doors.push(Door {
            side: WallSide::North,
            at: 5,
        });
        let layouts = [front, nook(4, -4)];
        let house = House::of(&layouts);
        // The plan moved down so its far corner is at the floor's.
        assert_eq!((house.rooms[0].x, house.rooms[0].y), (0, 4));
        assert_eq!((house.rooms[1].x, house.rooms[1].y), (4, 0));
        assert_eq!((house.width, house.depth), (8, 12));
        let between = house.wall(0, WallSide::North, 5).unwrap();
        assert_eq!(between.beyond, Some(1));
        assert_eq!(between.height, Height::Low);
        assert!(house.passable((5, 4), (5, 3)));
        assert!(!house.passable((6, 4), (6, 3)), "only through the doorway");
        assert!(
            !house.passable((4, 4), (5, 3)),
            "never diagonally through a wall"
        );
        assert_eq!(house.reachable(), vec![true, true]);
        // Every wall with nothing of the house behind it stands.
        for wall in house.walls.iter().filter(|wall| wall.beyond.is_none()) {
            assert_eq!(wall.height, Height::Full, "{wall:?}");
        }
        // The two rooms' cushions keep names of their own.
        let names: BTreeSet<u16> = house.pieces.iter().map(|piece| piece.uid).collect();
        assert_eq!(names.len(), house.pieces.len());
        let nook_cushion = house.named(1, 1).unwrap();
        assert_eq!(house.local(nook_cushion), Some((1, 1)));
        assert_eq!(
            house.piece(nook_cushion).map(|piece| (piece.x, piece.y)),
            Some((5, 1))
        );
    }

    #[test]
    fn a_wall_that_would_hide_a_room_tucked_behind_the_far_corner_is_cut_down() {
        let layouts = [front_room(), nook(-2, -4)];
        let house = House::of(&layouts);
        // The front room's west wall rises in front of the nook's corner where they meet, and
        // nowhere else.
        assert_eq!(
            house.wall(0, WallSide::West, 0).unwrap().height,
            Height::Low
        );
        assert_eq!(
            house.wall(0, WallSide::West, 5).unwrap().height,
            Height::Full
        );
        assert_eq!(
            house.wall(0, WallSide::North, 6).unwrap().height,
            Height::Full
        );
        assert_eq!(house.wall(0, WallSide::North, 0).unwrap().beyond, Some(1));
    }

    #[test]
    fn a_room_with_no_place_given_stands_past_the_others_and_cannot_be_reached_yet() {
        let mut lost = nook(0, 0);
        lost.plan = None;
        let house = House::of(&[front_room(), lost]);
        assert_eq!((house.rooms[1].x, house.rooms[1].y), (8, 0));
        assert_eq!(house.reachable(), vec![true, false]);
        // It stands at the side of the first room, so that wall between them is cut down.
        assert_eq!(house.wall(1, WallSide::West, 0).unwrap().beyond, Some(0));
    }

    #[test]
    fn spots_go_to_the_house_and_back_again() {
        let layouts = [front_room(), nook(8, 2)];
        let house = House::of(&layouts);
        let cushion = house.named(1, 1).unwrap();
        for (room, spot) in [
            (1, Spot::On { piece: 1, slot: 0 }),
            (
                0,
                Spot::Wall {
                    side: WallSide::West,
                    at: 2,
                },
            ),
            (1, Spot::Floor { x: 2, y: 3 }),
        ] {
            let at = house.at(room, spot).unwrap();
            assert_eq!(house.spot(at), Some((room, spot)));
        }
        assert_eq!(
            house.at(1, Spot::On { piece: 1, slot: 0 }),
            Some(At::On {
                piece: cushion,
                slot: 0
            })
        );
    }
}
