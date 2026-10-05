//! Finding the way across a room: from tile to tile over open floor, diagonally where both tiles
//! beside the corner are open too, so nobody squeezes between two pieces of furniture.

use crate::room;
use formiga_home_contract::RoomLayout;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The open floor of a room, row by row.
pub struct Floor {
    width: u8,
    depth: u8,
    open: Vec<bool>,
}

impl Floor {
    pub fn of(layout: &RoomLayout) -> Self {
        Self {
            width: layout.width,
            depth: layout.depth,
            open: room::walkable(layout),
        }
    }

    pub fn open(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < i32::from(self.width)
            && y < i32::from(self.depth)
            && self.open[(y * i32::from(self.width) + x) as usize]
    }

    /// The tile a point on the floor is in.
    pub fn tile_of(point: (f32, f32)) -> (i32, i32) {
        (point.0.floor() as i32, point.1.floor() as i32)
    }

    /// The open tile nearest `from`, by steps on the floor; `from` itself if it is open.
    pub fn nearest_open(&self, from: (i32, i32)) -> Option<(i32, i32)> {
        let reach = i32::from(self.width.max(self.depth));
        (0..=reach).find_map(|ring| {
            let mut best: Option<((i32, i32), i32)> = None;
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dy.abs()) != ring {
                        continue;
                    }
                    let tile = (from.0 + dx, from.1 + dy);
                    let cost = dx * dx + dy * dy;
                    if self.open(tile.0, tile.1) && best.is_none_or(|(_, c)| cost < c) {
                        best = Some((tile, cost));
                    }
                }
            }
            best.map(|(tile, _)| tile)
        })
    }

    /// The open tiles beside a block of tiles, nearest the given side first: where someone
    /// stands to use a piece, or to look at something on it.
    pub fn beside(&self, x: u8, y: u8, w: u8, d: u8, facing: (i32, i32)) -> Vec<(i32, i32)> {
        let (x0, y0, x1, y1) = (
            i32::from(x),
            i32::from(y),
            i32::from(x + w),
            i32::from(y + d),
        );
        let mut around = Vec::new();
        for ty in y0 - 1..=y1 {
            for tx in x0 - 1..=x1 {
                let inside = (x0..x1).contains(&tx) && (y0..y1).contains(&ty);
                let corner = !(x0..x1).contains(&tx) && !(y0..y1).contains(&ty);
                if !inside && !corner && self.open(tx, ty) {
                    around.push((tx, ty));
                }
            }
        }
        let centre = (
            (x0 + x1) as f32 / 2.0 + facing.0 as f32,
            (y0 + y1) as f32 / 2.0 + facing.1 as f32,
        );
        around.sort_by(|a, b| {
            let distance = |t: &(i32, i32)| {
                (t.0 as f32 + 0.5 - centre.0).powi(2) + (t.1 as f32 + 0.5 - centre.1).powi(2)
            };
            distance(a).total_cmp(&distance(b))
        });
        around
    }

    /// The way from one point on the floor to the middle of a tile, as tile centres after the
    /// first step; nothing if there is no way.
    pub fn route(&self, from: (f32, f32), to: (i32, i32)) -> Option<Vec<(f32, f32)>> {
        let start = Self::tile_of(from);
        let start = if self.open(start.0, start.1) {
            start
        } else {
            self.nearest_open(start)?
        };
        if !self.open(to.0, to.1) {
            return None;
        }
        let width = i32::from(self.width);
        let index = |(x, y): (i32, i32)| (y * width + x) as usize;
        let mut came_from = vec![usize::MAX; self.open.len()];
        let mut cost = vec![u32::MAX; self.open.len()];
        let mut frontier = BinaryHeap::new();
        cost[index(start)] = 0;
        frontier.push(Step {
            estimate: 0,
            tile: start,
        });
        let guess = |(x, y): (i32, i32)| {
            let (dx, dy) = ((x - to.0).abs(), (y - to.1).abs());
            (dx.max(dy) * 10 + dx.min(dy) * 4) as u32
        };
        while let Some(Step { tile, .. }) = frontier.pop() {
            if tile == to {
                break;
            }
            for (dx, dy) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let next = (tile.0 + dx, tile.1 + dy);
                if !self.open(next.0, next.1) {
                    continue;
                }
                let diagonal = dx != 0 && dy != 0;
                if diagonal && (!self.open(tile.0 + dx, tile.1) || !self.open(tile.0, tile.1 + dy))
                {
                    continue;
                }
                let step = if diagonal { 14 } else { 10 };
                let through = cost[index(tile)] + step;
                if through < cost[index(next)] {
                    cost[index(next)] = through;
                    came_from[index(next)] = index(tile);
                    frontier.push(Step {
                        estimate: through + guess(next),
                        tile: next,
                    });
                }
            }
        }
        if cost[index(to)] == u32::MAX {
            return None;
        }
        let mut tiles = vec![to];
        let mut at = index(to);
        while at != index(start) {
            at = came_from[at];
            tiles.push(((at as i32) % width, (at as i32) / width));
        }
        tiles.reverse();
        let mut waypoints: Vec<_> = tiles
            .into_iter()
            .map(|(x, y)| (x as f32 + 0.5, y as f32 + 0.5))
            .collect();
        // Already in the first tile: walk straight on from where it stands.
        if waypoints.len() > 1 {
            waypoints.remove(0);
        }
        Some(waypoints)
    }
}

#[derive(PartialEq, Eq)]
struct Step {
    estimate: u32,
    tile: (i32, i32),
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimate
            .cmp(&self.estimate)
            .then_with(|| self.tile.cmp(&other.tile))
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starter;
    use formiga_home_contract::sample;

    #[test]
    fn the_way_goes_round_furniture_and_never_through_it() {
        let layout = starter::room(&sample::snapshot());
        let floor = Floor::of(&layout);
        let route = floor.route((0.5, 7.5), (7, 1)).expect("a way across");
        for &(x, y) in &route {
            assert!(
                floor.open(x as i32, y as i32),
                "({x}, {y}) is not open floor"
            );
        }
        assert_eq!(route.last(), Some(&(7.5, 1.5)));
    }

    #[test]
    fn there_is_no_way_onto_furniture_and_someone_on_it_steps_off_first() {
        let layout = starter::room(&sample::snapshot());
        let floor = Floor::of(&layout);
        let bed = layout
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "bed")
            .unwrap();
        assert!(
            floor
                .route((4.5, 4.5), (i32::from(bed.x), i32::from(bed.y)))
                .is_none()
        );
        let from_bed = floor.route((f32::from(bed.x) + 0.5, 0.5), (4, 4));
        assert!(
            from_bed.is_some(),
            "someone on the bed can still get off it"
        );
        let beside = floor.beside(bed.x, bed.y, 1, 2, (0, 1));
        assert!(!beside.is_empty());
        assert!(beside.iter().all(|&(x, y)| floor.open(x, y)));
    }
}
