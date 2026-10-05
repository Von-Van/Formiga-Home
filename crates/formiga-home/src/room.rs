//! What can go where in a room, by simple footprints on whole tiles rather than by collision: a
//! piece takes the tiles under it, a rug lies under anything, a thing shown on the floor takes a
//! tile of its own, and every surface and stretch of wall holds one thing at a time.

use crate::catalog::{self, Holds, Piece};
use formiga_home_contract::limits::MAX_PLACED_PER_HOUSEHOLD;
use formiga_home_contract::{
    DisplayId, DisplayItem, DisplayMode, HouseholdHome, PlacedDisplay, PlacedPiece, RoomLayout,
    Spot, WallSide,
};

/// A block of floor tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub d: u8,
}

impl Footprint {
    pub fn contains(&self, x: u8, y: u8) -> bool {
        (self.x..self.x + self.w).contains(&x) && (self.y..self.y + self.d).contains(&y)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.d
            && other.y < self.y + self.d
    }

    pub fn tiles(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        (self.y..self.y + self.d).flat_map(move |y| (self.x..self.x + self.w).map(move |x| (x, y)))
    }

    /// Its middle, in tiles.
    pub fn centre(&self) -> (f32, f32) {
        (
            f32::from(self.x) + f32::from(self.w) / 2.0,
            f32::from(self.y) + f32::from(self.d) / 2.0,
        )
    }
}

/// The tiles a placed piece takes. A piece this build does not know takes its one tile.
pub fn footprint(placed: &PlacedPiece) -> Footprint {
    let (w, d) = catalog::piece(&placed.piece).map_or((1, 1), |piece| piece.size_at(placed.turn));
    Footprint {
        x: placed.x,
        y: placed.y,
        w,
        d,
    }
}

/// Whether a placed piece is one that is walked over and stood on.
fn is_flat(placed: &PlacedPiece) -> bool {
    catalog::piece(&placed.piece).is_some_and(|piece| piece.flat)
}

/// How something is shown in a spot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Showing {
    /// As itself: the shell on the shelf, the postcard pinned up.
    Itself,
    /// On its card, plaque or frame: always possible on a surface or a wall.
    Card,
}

/// What kind of place a spot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Top,
    Shelf,
    Wall,
    Floor,
}

/// How `item` would be shown in a place of this kind, if it can be shown there at all. Anything
/// can go on a surface or a wall, on its card if not as itself; only what stands on the floor
/// goes on the floor.
pub fn showing(item: &DisplayItem, place: Place) -> Option<Showing> {
    let itself = |modes: &[DisplayMode]| modes.iter().any(|mode| item.allows(*mode));
    match place {
        Place::Top if itself(&[DisplayMode::SurfaceSmall]) => Some(Showing::Itself),
        Place::Shelf if itself(&[DisplayMode::SurfaceSmall, DisplayMode::Case]) => {
            Some(Showing::Itself)
        }
        Place::Wall if itself(&[DisplayMode::Wall, DisplayMode::Textile]) => Some(Showing::Itself),
        Place::Floor if itself(&[DisplayMode::Floor]) => Some(Showing::Itself),
        Place::Floor => None,
        _ if item.allows(DisplayMode::FallbackCard) => Some(Showing::Card),
        _ => None,
    }
}

/// One surface of a placed piece, where it is now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceAt {
    pub slot: u8,
    /// On the floor, in tiles.
    pub at: (f32, f32),
    pub height: i32,
    pub holds: Holds,
}

/// Every surface a placed piece has, where it stands.
pub fn surfaces(placed: &PlacedPiece) -> Vec<SurfaceAt> {
    let Some(piece) = catalog::piece(&placed.piece) else {
        return Vec::new();
    };
    piece
        .surfaces
        .iter()
        .enumerate()
        .map(|(slot, surface)| {
            let (x, y) = catalog::turned(surface.at, piece.size, placed.turn);
            SurfaceAt {
                slot: slot as u8,
                at: (f32::from(placed.x) + x, f32::from(placed.y) + y),
                height: surface.height,
                holds: surface.holds,
            }
        })
        .collect()
}

/// The kind of place a spot is in this room, if the room has it.
pub fn place_of(layout: &RoomLayout, spot: Spot) -> Option<Place> {
    match spot {
        Spot::On { piece, slot } => {
            let surface = surfaces(layout.piece(piece)?)
                .into_iter()
                .nth(usize::from(slot))?;
            Some(match surface.holds {
                Holds::Top => Place::Top,
                Holds::Shelf => Place::Shelf,
            })
        }
        Spot::Wall {
            side: WallSide::North,
            at,
        } => (at < layout.width).then_some(Place::Wall),
        Spot::Wall {
            side: WallSide::West,
            at,
        } => (at < layout.depth).then_some(Place::Wall),
        Spot::Floor { x, y } => (x < layout.width && y < layout.depth).then_some(Place::Floor),
    }
}

/// What is shown in a spot, if anything is.
pub fn shown_at(layout: &RoomLayout, spot: Spot) -> Option<&DisplayId> {
    layout
        .displays
        .iter()
        .find(|shown| shown.spot == spot)
        .map(|shown| &shown.item)
}

/// Whether a tile has something standing on it that nobody can walk through or put a piece on.
#[cfg(test)]
pub fn blocked(layout: &RoomLayout, x: u8, y: u8) -> bool {
    layout
        .pieces
        .iter()
        .any(|placed| !is_flat(placed) && footprint(placed).contains(x, y))
        || layout
            .displays
            .iter()
            .any(|shown| shown.spot == Spot::Floor { x, y })
}

/// Whether `piece` can stand at `(x, y)` turned `turn`, leaving out the piece `moving` (which is
/// the one being moved, if any): inside the room, and over nothing but rugs, unless it is a rug,
/// which can lie under anything but another rug.
pub fn can_place(
    layout: &RoomLayout,
    piece: &Piece,
    x: u8,
    y: u8,
    turn: u8,
    moving: Option<u16>,
) -> bool {
    let (w, d) = piece.size_at(turn);
    if u16::from(x) + u16::from(w) > u16::from(layout.width)
        || u16::from(y) + u16::from(d) > u16::from(layout.depth)
    {
        return false;
    }
    let here = Footprint { x, y, w, d };
    let clashes = layout
        .pieces
        .iter()
        .filter(|placed| Some(placed.uid) != moving)
        .any(|placed| is_flat(placed) == piece.flat && footprint(placed).overlaps(&here));
    let on_a_find = !piece.flat
        && layout
            .displays
            .iter()
            .any(|shown| matches!(shown.spot, Spot::Floor { x, y } if here.contains(x, y)));
    !clashes && !on_a_find
}

/// How `item` would be shown at `spot`, if it can go there now: the room has the spot, nothing
/// else is shown there, and it is a way the item may be shown. A floor spot must also be clear.
pub fn can_show(layout: &RoomLayout, item: &DisplayItem, spot: Spot) -> Option<Showing> {
    let place = place_of(layout, spot)?;
    if shown_at(layout, spot).is_some_and(|there| there != &item.id) {
        return None;
    }
    if let Spot::Floor { x, y } = spot {
        let piece_there = layout
            .pieces
            .iter()
            .any(|placed| !is_flat(placed) && footprint(placed).contains(x, y));
        if piece_there {
            return None;
        }
    }
    showing(item, place)
}

/// The next name free for a piece anywhere in the house, so that every piece is named once
/// across all its rooms.
pub fn next_uid(home: &HouseholdHome) -> u16 {
    home.rooms
        .iter()
        .flat_map(|layout| layout.pieces.iter().map(|placed| placed.uid))
        .max()
        .map_or(1, |uid| uid.saturating_add(1))
}

/// Whether the household has room for one more thing.
pub fn has_room_for_more(home: &HouseholdHome) -> bool {
    home.placed() < MAX_PLACED_PER_HOUSEHOLD
}

/// Put a new piece in one of the house's rooms. Its name.
pub fn add_piece(
    home: &mut HouseholdHome,
    room: usize,
    piece: &Piece,
    x: u8,
    y: u8,
    turn: u8,
) -> u16 {
    let uid = next_uid(home);
    if let Some(layout) = home.rooms.get_mut(room) {
        layout.pieces.push(PlacedPiece {
            uid,
            piece: piece.catalog_id(),
            x,
            y,
            turn,
        });
    }
    uid
}

/// Take a piece out of the room, and whatever was shown on it with it. What was shown on it.
pub fn remove_piece(layout: &mut RoomLayout, uid: u16) -> Vec<DisplayId> {
    layout.pieces.retain(|placed| placed.uid != uid);
    let mut freed = Vec::new();
    layout.displays.retain(|shown| match shown.spot {
        Spot::On { piece, .. } if piece == uid => {
            freed.push(shown.item.clone());
            false
        }
        _ => true,
    });
    freed
}

/// Show `item` at `spot`, wherever it was shown in this house before.
pub fn show(home: &mut HouseholdHome, room: usize, item: &DisplayId, spot: Spot) {
    home.take_down(item);
    if let Some(layout) = home.rooms.get_mut(room) {
        layout.displays.push(PlacedDisplay {
            item: item.clone(),
            spot,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starter;
    use formiga_home_contract::sample;

    fn item(id: DisplayId) -> DisplayItem {
        sample::snapshot().item(&id).unwrap().clone()
    }

    #[test]
    fn a_piece_cannot_stand_on_another_but_a_rug_lies_under_anything() {
        let snapshot = sample::snapshot();
        let layout = starter::room(&snapshot);
        let armchair = catalog::PIECES.iter().find(|p| p.id == "armchair").unwrap();
        let rug = catalog::PIECES
            .iter()
            .find(|p| p.id == "round_rug")
            .unwrap();
        let bed = layout
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "bed")
            .unwrap();
        assert!(!can_place(&layout, armchair, bed.x, bed.y, 0, None));
        assert!(can_place(&layout, armchair, bed.x, bed.y, 0, Some(bed.uid)));
        let placed_rug = layout
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "round_rug")
            .unwrap();
        assert!(can_place(
            &layout,
            armchair,
            placed_rug.x,
            placed_rug.y,
            0,
            None
        ));
        assert!(!can_place(
            &layout,
            rug,
            placed_rug.x,
            placed_rug.y,
            0,
            None
        ));
        assert!(
            !can_place(&layout, armchair, layout.width, 0, 0, None),
            "off the floor"
        );
        let sofa = catalog::PIECES.iter().find(|p| p.id == "sofa").unwrap();
        assert!(
            !can_place(&layout, sofa, layout.width - 1, 4, 0, None),
            "half off the floor"
        );
        assert!(
            can_place(&layout, sofa, layout.width - 1, 4, 1, None),
            "turned, it fits"
        );
    }

    #[test]
    fn anything_can_be_shown_on_a_card_but_only_floor_things_stand_on_the_floor() {
        let shell = item(DisplayId::find(3));
        let postcard = item(DisplayId::find(76));
        let sheep = item(DisplayId::find(132));
        assert_eq!(showing(&shell, Place::Top), Some(Showing::Itself));
        assert_eq!(showing(&shell, Place::Wall), Some(Showing::Card));
        assert_eq!(showing(&shell, Place::Floor), None);
        assert_eq!(showing(&postcard, Place::Wall), Some(Showing::Itself));
        assert_eq!(showing(&sheep, Place::Floor), Some(Showing::Itself));
        let ribbon = item(DisplayId::souvenir("picnic_ribbon").unwrap());
        assert_eq!(showing(&ribbon, Place::Wall), Some(Showing::Itself));
    }

    #[test]
    fn one_spot_holds_one_thing_and_a_find_on_the_floor_takes_its_tile() {
        let snapshot = sample::snapshot();
        let mut home = starter::home(&snapshot);
        let sheep = item(DisplayId::find(132));
        let jar = item(DisplayId::find(9));
        let spot = Spot::Floor { x: 6, y: 6 };
        assert_eq!(
            can_show(&home.rooms[0], &sheep, spot),
            Some(Showing::Itself)
        );
        show(&mut home, 0, &sheep.id, spot);
        assert_eq!(can_show(&home.rooms[0], &jar, spot), None);
        assert!(blocked(&home.rooms[0], 6, 6));
        let house = crate::house::House::of(&home.rooms);
        assert!(!house.walkable()[6 * 8 + 6]);
        let armchair = catalog::PIECES.iter().find(|p| p.id == "armchair").unwrap();
        assert!(!can_place(&home.rooms[0], armchair, 6, 6, 0, None));
        // Shown again somewhere else in the house, it moves rather than doubling.
        show(&mut home, 0, &sheep.id, Spot::Floor { x: 5, y: 6 });
        assert_eq!(home.shown().filter(|id| **id == sheep.id).count(), 1);
    }

    #[test]
    fn taking_a_piece_away_takes_down_what_was_shown_on_it() {
        let snapshot = sample::snapshot();
        let mut home = starter::home(&snapshot);
        let shelf = home.rooms[0]
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "shelf")
            .unwrap()
            .uid;
        let shell = DisplayId::find(3);
        show(
            &mut home,
            0,
            &shell,
            Spot::On {
                piece: shelf,
                slot: 2,
            },
        );
        assert_eq!(remove_piece(&mut home.rooms[0], shelf), vec![shell]);
        assert!(home.rooms[0].displays.is_empty());
    }
}
