//! How a house looks inside the first time it is opened: nearly empty, so that what comes to be in
//! it is the household's own. A bed against the far wall, a basket beside it for each little one, a
//! chair, a rug, a side table and an empty shelf, in the floor and walls the house's outside hints
//! at. Everything else is in the catalogue.

use crate::catalog;
use formiga_home_contract::{CatalogId, HomeSnapshot, HouseholdHome, PlacedPiece, RoomLayout};
use formiga_travel::TravelRole;

/// The starter room's size, in tiles.
pub const ROOM_TILES: u8 = 8;

/// The starter room for the household `snapshot` opens.
pub fn room(snapshot: &HomeSnapshot) -> RoomLayout {
    let (floor, wall) = catalog::finishes_for(snapshot.household.style);
    let mut pieces = vec![
        ("bed", 1, 0, 0),
        ("side_table", 2, 0, 0),
        ("shelf", 4, 0, 0),
        ("fern", 0, 0, 1),
        ("round_rug", 3, 3, 0),
        ("armchair", 0, 5, 1),
        ("ball", 5, 5, 0),
    ];
    let little_ones = snapshot
        .residents
        .iter()
        .filter(|resident| matches!(resident.role, TravelRole::Mini { .. }))
        .count();
    // Baskets along the far wall, past the shelf, one for each little one.
    for x in (5..ROOM_TILES).take(little_ones) {
        pieces.push(("basket", x, 0, 0));
    }
    RoomLayout {
        width: ROOM_TILES,
        depth: ROOM_TILES,
        floor: CatalogId::known(floor),
        wall: CatalogId::known(wall),
        pieces: pieces
            .into_iter()
            .enumerate()
            .map(|(index, (piece, x, y, turn))| PlacedPiece {
                uid: index as u16 + 1,
                piece: CatalogId::known(piece),
                x,
                y,
                turn,
            })
            .collect(),
        displays: Vec::new(),
    }
}

/// The household's home before anything has been arranged in it.
pub fn home(snapshot: &HomeSnapshot) -> HouseholdHome {
    HouseholdHome {
        keeper: snapshot.household.keeper,
        rooms: vec![room(snapshot)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room;
    use formiga_home_contract::{HomeDocument, HomeState, sample};

    #[test]
    fn the_starter_room_is_a_room_that_could_be_arranged() {
        let snapshot = sample::snapshot();
        let layout = room(&snapshot);
        for (index, placed) in layout.pieces.iter().enumerate() {
            let piece = catalog::piece(&placed.piece).expect("a piece the catalogue has");
            let mut others = layout.clone();
            others.pieces.remove(index);
            assert!(
                room::can_place(&others, piece, placed.x, placed.y, placed.turn, None),
                "{} does not fit where it starts",
                piece.id
            );
        }
        let mut state = HomeState::new(&snapshot.colony_key);
        state.set_household(home(&snapshot));
        state.validate().unwrap();
        let baskets = layout
            .pieces
            .iter()
            .filter(|placed| placed.piece.as_str() == "basket")
            .count();
        assert_eq!(baskets, 1, "one for Pip");
    }

    #[test]
    fn there_is_room_to_walk_from_every_open_tile_to_every_other() {
        let layout = room(&sample::snapshot());
        let open = room::walkable(&layout);
        let width = usize::from(layout.width);
        let start = open.iter().position(|free| *free).unwrap();
        let mut seen = vec![false; open.len()];
        let mut frontier = vec![start];
        seen[start] = true;
        while let Some(at) = frontier.pop() {
            let (x, y) = (at % width, at / width);
            let neighbours = [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ];
            for (nx, ny) in neighbours {
                if nx < width && ny < usize::from(layout.depth) {
                    let next = ny * width + nx;
                    if open[next] && !seen[next] {
                        seen[next] = true;
                        frontier.push(next);
                    }
                }
            }
        }
        let unreachable = open
            .iter()
            .zip(&seen)
            .filter(|(free, seen)| **free && !**seen)
            .count();
        assert_eq!(unreachable, 0);
    }
}
