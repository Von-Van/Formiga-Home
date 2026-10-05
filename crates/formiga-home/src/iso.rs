//! The house's geometry: a fixed 2:1 isometric grid, seen from the front corner, with the far
//! walls standing and the near ones cut away.
//!
//! Floor positions are in tiles, `x` along the house's width (down to the right on screen) and
//! `y` along its depth (down to the left). A tile is 32 pixels across and 16 down. Everything in
//! the house is placed on this grid; the companions are drawn upright over it, anchored by their
//! feet.

/// A floor tile's width and height on screen.
pub const TILE_W: i32 = 32;
pub const TILE_H: i32 = 16;
const HALF_W: f32 = (TILE_W / 2) as f32;
const HALF_H: f32 = (TILE_H / 2) as f32;

/// The smallest picture a house is drawn into, before it is scaled up whole for the window. One
/// room eight tiles a side fills it; a bigger house is drawn into a bigger picture of the same
/// shape, which the window shows smaller.
pub const SCENE_WIDTH: u32 = 320;
pub const SCENE_HEIGHT: u32 = 216;
/// Room left round the house in its picture, across and down.
const MARGIN: (f32, f32) = (32.0, 13.0);

/// How tall the far walls stand above the floor.
pub const WALL_HEIGHT: i32 = 56;
/// How thick the floor is drawn at the cut-away front edges: the dollhouse's cross-section.
pub const SLAB: i32 = 5;

/// Where a house of a given size sits in its picture, and how big the picture is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub width: u8,
    pub depth: u8,
    /// The far corner of the house's floor, on screen.
    pub origin: (f32, f32),
    /// The picture's size, in pixels.
    pub size: (u32, u32),
}

impl View {
    /// A house centred in a picture just big enough for it, the wall tops and the floor's front
    /// edge both inside.
    pub fn new(width: u8, depth: u8) -> Self {
        let (w, d) = (f32::from(width), f32::from(depth));
        let floor_height = (w + d) * HALF_H + SLAB as f32;
        let extent = floor_height + WALL_HEIGHT as f32;
        let grow = (((w + d) * HALF_W + 2.0 * MARGIN.0) / SCENE_WIDTH as f32)
            .max((extent + 2.0 * MARGIN.1) / SCENE_HEIGHT as f32)
            .max(1.0);
        let size = (
            (SCENE_WIDTH as f32 * grow).ceil() as u32,
            (SCENE_HEIGHT as f32 * grow).ceil() as u32,
        );
        let top = ((size.1 as f32 - extent) / 2.0).floor() + WALL_HEIGHT as f32 + 2.0;
        let centre = (size.0 as f32 / 2.0 - (w - d) * HALF_W / 2.0).floor();
        Self {
            width,
            depth,
            origin: (centre, top),
            size,
        }
    }

    /// A house centred in a picture just big enough for its rooms: an L of rooms needs less
    /// picture than the rectangle round it, and is drawn the bigger for it.
    pub fn of(house: &crate::house::House) -> Self {
        let (mut left, mut right, mut top, mut bottom) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for room in &house.rooms {
            let (x, y) = (f32::from(room.x), f32::from(room.y));
            let (w, d) = (f32::from(room.width), f32::from(room.depth));
            for (fx, fy) in [(x, y), (x + w, y), (x + w, y + d), (x, y + d)] {
                let (sx, sy) = ((fx - fy) * HALF_W, (fx + fy) * HALF_H);
                left = left.min(sx);
                right = right.max(sx);
                top = top.min(sy - WALL_HEIGHT as f32);
                bottom = bottom.max(sy + SLAB as f32);
            }
        }
        if house.rooms.is_empty() {
            return Self::new(house.width, house.depth);
        }
        let (wide, tall) = (right - left, bottom - top);
        let grow = ((wide + 2.0 * MARGIN.0) / SCENE_WIDTH as f32)
            .max((tall + 2.0 * MARGIN.1) / SCENE_HEIGHT as f32)
            .max(1.0);
        let size = (
            (SCENE_WIDTH as f32 * grow).ceil() as u32,
            (SCENE_HEIGHT as f32 * grow).ceil() as u32,
        );
        Self {
            width: house.width,
            depth: house.depth,
            origin: (
                ((size.0 as f32 - wide) / 2.0).floor() - left,
                ((size.1 as f32 - tall) / 2.0).floor() - top + 2.0,
            ),
            size,
        }
    }

    /// A floor position on screen.
    pub fn screen(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin.0 + (x - y) * HALF_W,
            self.origin.1 + (x + y) * HALF_H,
        )
    }

    /// The same, to the pixel.
    pub fn pixel(&self, x: f32, y: f32) -> (i32, i32) {
        let (sx, sy) = self.screen(x, y);
        (sx.round() as i32, sy.round() as i32)
    }

    /// The floor position under a point on screen, which may be outside the room.
    pub fn floor_at(&self, sx: f32, sy: f32) -> (f32, f32) {
        let u = (sx - self.origin.0) / HALF_W;
        let v = (sy - self.origin.1) / HALF_H;
        ((u + v) / 2.0, (v - u) / 2.0)
    }

    /// The tile under a point on screen, if it is inside the room.
    pub fn tile_at(&self, sx: f32, sy: f32) -> Option<(u8, u8)> {
        let (x, y) = self.floor_at(sx, sy);
        (x >= 0.0 && y >= 0.0 && x < f32::from(self.width) && y < f32::from(self.depth))
            .then_some((x as u8, y as u8))
    }

    /// The four corners of a block of tiles on the floor, back, right, front, left.
    pub fn footprint(&self, x: u8, y: u8, w: u8, d: u8) -> [(f32, f32); 4] {
        let (x0, y0) = (f32::from(x), f32::from(y));
        let (x1, y1) = (x0 + f32::from(w), y0 + f32::from(d));
        [
            self.screen(x0, y0),
            self.screen(x1, y0),
            self.screen(x1, y1),
            self.screen(x0, y1),
        ]
    }

    /// The middle of a tile, where someone standing on it puts their feet.
    pub fn tile_centre(&self, x: u8, y: u8) -> (f32, f32) {
        self.screen(f32::from(x) + 0.5, f32::from(y) + 0.5)
    }

    /// Where something hung on a stretch of wall is centred, `height` pixels above the floor.
    pub fn on_wall(&self, wall: &crate::house::Wall, height: i32) -> (i32, i32) {
        let (x, y) = wall.middle();
        let (sx, sy) = self.screen(x, y);
        (sx.round() as i32, (sy - height as f32).round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_the_floor_comes_back_from_the_screen_where_it_was() {
        let view = View::new(8, 8);
        for (x, y) in [(0.0, 0.0), (3.25, 5.5), (7.9, 0.1), (8.0, 8.0)] {
            let (sx, sy) = view.screen(x, y);
            let (bx, by) = view.floor_at(sx, sy);
            assert!((bx - x).abs() < 1e-4 && (by - y).abs() < 1e-4);
        }
        let (cx, cy) = view.tile_centre(2, 6);
        assert_eq!(view.tile_at(cx, cy), Some((2, 6)));
        assert_eq!(view.tile_at(view.origin.0, view.origin.1 - 4.0), None);
    }

    #[test]
    fn the_whole_house_and_its_walls_fit_its_picture() {
        for (w, d) in [(8, 8), (6, 6), (10, 8), (4, 12), (16, 12), (24, 20)] {
            let view = View::new(w, d);
            let corners = view.footprint(0, 0, w, d);
            for (sx, sy) in corners {
                assert!(sx >= 0.0 && sx <= view.size.0 as f32, "{w}x{d}: {sx}");
                assert!(sy <= (view.size.1 as i32 - SLAB) as f32, "{w}x{d}: {sy}");
            }
            assert!(
                view.origin.1 - WALL_HEIGHT as f32 >= 0.0,
                "{w}x{d}: walls cut off"
            );
            // Always the one shape, so the window lays out the same however big the house.
            let shape = view.size.0 as f32 / view.size.1 as f32;
            assert!((shape - SCENE_WIDTH as f32 / SCENE_HEIGHT as f32).abs() < 0.01);
        }
        // One room eight tiles a side is drawn where it always was.
        let one = View::new(8, 8);
        assert_eq!(one.size, (SCENE_WIDTH, SCENE_HEIGHT));
        assert_eq!(one.origin, (160.0, 71.0));
        let layout = crate::starter::room(&formiga_home_contract::sample::snapshot());
        let house = crate::house::House::of(std::slice::from_ref(&layout));
        assert_eq!(View::of(&house), one);
    }
}
