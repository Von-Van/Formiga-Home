//! Everything Home draws that is not a companion: the room's floor and walls, every piece of
//! furniture at every turn, everything the colony shows, and the little signs over a resident's
//! head. Companions are drawn by `formiga-art`, from the snapshot, never redrawn here.
//!
//! Furniture is built from blocks on its own footprint, so every piece shares one light and one
//! grain: tops catch the light from the upper left, the faces that look left are lit, the faces
//! that look right are in shade, and every piece ends with an edge in a darker shade of its own
//! colours. A piece is drawn once from the front and once from behind; its two other turns are
//! those seen in a mirror.

pub mod cues;
pub mod displays;
pub mod furniture;
pub mod shell;

use crate::catalog::Piece;
use crate::iso::{TILE_H, TILE_W};
use crate::paint::{self, Ramp};
use formiga_art::Canvas;
use std::collections::HashMap;

/// A drawing with the point it hangs from: for furniture, the far corner of its footprint.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub canvas: Canvas,
    pub anchor: (i32, i32),
    /// What of it stands in front of whatever is shown on it or using it: a shelf's front post,
    /// a case's glass. Drawn after them, at the same anchor.
    pub over: Option<Canvas>,
}

impl Sprite {
    /// Where its top-left goes for its anchor to land on `at`.
    pub fn origin(&self, at: (i32, i32)) -> (i32, i32) {
        (at.0 - self.anchor.0, at.1 - self.anchor.1)
    }

    /// The same drawing seen in a mirror, anchored at the same point of what it shows.
    pub fn mirrored(&self) -> Self {
        let flip = |canvas: &Canvas| {
            let mut flipped = canvas.clone();
            flipped.mirror_horizontal();
            flipped
        };
        Self {
            canvas: flip(&self.canvas),
            anchor: (self.canvas.width() as i32 - self.anchor.0, self.anchor.1),
            over: self.over.as_ref().map(flip),
        }
    }

    /// Whether the drawing covers this point of the scene, drawn at `at`.
    pub fn covers(&self, at: (i32, i32), point: (i32, i32)) -> bool {
        let (ox, oy) = self.origin(at);
        let (x, y) = (point.0 - ox, point.1 - oy);
        self.canvas.get(x, y).a > 40 || self.over.as_ref().is_some_and(|over| over.get(x, y).a > 40)
    }
}

/// A box on a piece's footprint: tiles across and deep from the footprint's far corner, and pixels
/// up from the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub x: (f32, f32),
    pub y: (f32, f32),
    pub z: (f32, f32),
    pub ramp: Ramp,
}

impl Block {
    pub const fn new(x: (f32, f32), y: (f32, f32), z: (f32, f32), ramp: Ramp) -> Self {
        Self { x, y, z, ramp }
    }

    /// The same box with the footprint turned round to face away: what was at the front is at the
    /// back.
    pub fn facing_away(self, depth: f32) -> Self {
        Self {
            y: (depth - self.y.1, depth - self.y.0),
            ..self
        }
    }

    fn overlaps_on_floor(&self, other: &Self) -> bool {
        self.x.0 < other.x.1 && other.x.0 < self.x.1 && self.y.0 < other.y.1 && other.y.0 < self.y.1
    }
}

/// Draws blocks and shapes onto one piece's canvas, from the piece's own floor coordinates.
pub struct Easel {
    pub canvas: Canvas,
    pub anchor: (i32, i32),
    pub salt: u32,
}

/// How much room a piece's canvas leaves round it, for its shadow and its edge.
const MARGIN: i32 = 6;

impl Easel {
    /// A canvas for a footprint `w` by `d` tiles, standing up to `height` pixels.
    pub fn new(w: u8, d: u8, height: i32, salt: u32) -> Self {
        let (w, d) = (i32::from(w), i32::from(d));
        let width = (w + d) * TILE_W / 2 + MARGIN * 2;
        let tall = height + MARGIN * 2 + 6;
        let full = (w + d) * TILE_H / 2 + tall + MARGIN;
        Self {
            canvas: Canvas::new(width as u32, full as u32),
            anchor: (d * TILE_W / 2 + MARGIN, tall),
            salt,
        }
    }

    /// A point of the piece on the canvas.
    pub fn at(&self, x: f32, y: f32, z: f32) -> (f32, f32) {
        (
            self.anchor.0 as f32 + (x - y) * (TILE_W / 2) as f32,
            self.anchor.1 as f32 + (x + y) * (TILE_H / 2) as f32 - z,
        )
    }

    pub fn pixel(&self, x: f32, y: f32, z: f32) -> (i32, i32) {
        let (px, py) = self.at(x, y, z);
        (px.round() as i32, py.round() as i32)
    }

    /// One box: its top in the light, its left face lit, its right face in shade, and a fleck of
    /// grain over all three.
    pub fn block(&mut self, block: Block) {
        let Block { x, y, z, ramp } = block;
        let corner = |cx: f32, cy: f32, cz: f32| self.at(cx, cy, cz);
        let top = [
            corner(x.0, y.0, z.1),
            corner(x.1, y.0, z.1),
            corner(x.1, y.1, z.1),
            corner(x.0, y.1, z.1),
        ];
        let left = [
            corner(x.0, y.1, z.1),
            corner(x.1, y.1, z.1),
            corner(x.1, y.1, z.0),
            corner(x.0, y.1, z.0),
        ];
        let right = [
            corner(x.1, y.0, z.1),
            corner(x.1, y.1, z.1),
            corner(x.1, y.1, z.0),
            corner(x.1, y.0, z.0),
        ];
        let salt = self.salt;
        let canvas = &mut self.canvas;
        let grain = |canvas: &mut Canvas, points: &[(f32, f32)], tone: formiga_art::Rgba, fleck| {
            paint::fill_polygon(points, |px, py| {
                let speckle = paint::chance(px, py, salt, 22);
                let color = if speckle { fleck } else { tone };
                canvas.set(px, py, color);
            });
        };
        if z.1 > z.0 {
            grain(
                canvas,
                &left,
                ramp.base,
                paint::mix(ramp.base, ramp.shadow, 0.35),
            );
            grain(
                canvas,
                &right,
                ramp.shadow,
                paint::mix(ramp.shadow, ramp.edge, 0.3),
            );
        }
        grain(
            canvas,
            &top,
            ramp.light,
            paint::mix(ramp.light, ramp.base, 0.4),
        );
        // The lit edge where the top meets the left face, and the shaded corner below it.
        let (front, left_end, right_end) = (
            self.pixel(x.1, y.1, z.1),
            self.pixel(x.0, y.1, z.1),
            self.pixel(x.1, y.0, z.1),
        );
        paint::line(&mut self.canvas, left_end, front, ramp.shine);
        paint::line(
            &mut self.canvas,
            front,
            right_end,
            paint::mix(ramp.light, ramp.shine, 0.4),
        );
        if z.1 - z.0 >= 3.0 {
            let bottom = self.pixel(x.1, y.1, z.0);
            paint::line(
                &mut self.canvas,
                (front.0, front.1 + 1),
                (bottom.0, bottom.1 - 1),
                ramp.edge,
            );
        }
    }

    /// Several boxes, back to front: a box over another's footprint goes after it if it stands
    /// higher, and boxes side by side go in the order they stand from the far corner.
    pub fn blocks(&mut self, mut blocks: Vec<Block>) {
        blocks.sort_by(|a, b| {
            if a.overlaps_on_floor(b) {
                a.z.0.total_cmp(&b.z.0)
            } else {
                let depth = |block: &Block| block.x.0 + block.x.1 + block.y.0 + block.y.1;
                depth(a).total_cmp(&depth(b))
            }
        });
        for block in blocks {
            self.block(block);
        }
    }

    /// A soft shadow on the floor round a piece's footprint.
    pub fn shadow(&mut self, x: (f32, f32), y: (f32, f32), strength: u8) {
        let points = [
            self.at(x.0, y.0, 0.0),
            self.at(x.1, y.0, 0.0),
            self.at(x.1, y.1, 0.0),
            self.at(x.0, y.1, 0.0),
        ];
        let shade = paint::rgba(0x2a1a14, strength);
        let canvas = &mut self.canvas;
        paint::fill_polygon(&points, |px, py| paint::put(canvas, px, py, shade));
        // Feathered by a pixel to the front, where the light falls past it.
        let soft = paint::rgba(0x2a1a14, strength / 2);
        let front = [
            self.at(x.0 - 0.06, y.1, 0.0),
            self.at(x.1 + 0.06, y.1 + 0.06, 0.0),
            self.at(x.1 + 0.06, y.0 - 0.06, 0.0),
        ];
        let canvas = &mut self.canvas;
        paint::fill_polygon(&front, |px, py| {
            if canvas.get(px, py).a == 0 {
                paint::put(canvas, px, py, soft);
            }
        });
    }

    /// An upright round thing's body between two heights: a pot, a basket, a lamp's shade. Shaded
    /// across from lit on the left to shade on the right.
    pub fn cylinder(&mut self, centre: (f32, f32), radius: (f32, f32), z: (f32, f32), ramp: Ramp) {
        let (bottom_r, top_r) = radius;
        let (cx, base) = self.at(centre.0, centre.1, z.0);
        let height = z.1 - z.0;
        let rows = height.ceil() as i32;
        for row in 0..=rows {
            let along = row as f32 / rows.max(1) as f32;
            let r = bottom_r + (top_r - bottom_r) * along;
            let cy = base - height * along;
            let span = r.round() as i32;
            for dx in -span..=span {
                let across = (dx as f32 / r.max(1.0) + 1.0) / 2.0;
                let tone = if across < 0.3 {
                    ramp.light
                } else if across < 0.68 {
                    ramp.base
                } else {
                    ramp.shadow
                };
                // The near half of the ellipse's rim at this height.
                let dy = (r / 2.0) * (1.0 - (dx as f32 / r.max(1.0)).powi(2)).max(0.0).sqrt();
                let (px, py) = (cx.round() as i32 + dx, (cy + dy).round() as i32);
                let fleck = paint::chance(px, py, self.salt, 20);
                let color = if fleck {
                    paint::mix(tone, ramp.shadow, 0.3)
                } else {
                    tone
                };
                self.canvas.set(px, py, color);
                self.canvas.set(px, py - 1, color);
            }
        }
    }

    /// A flat ellipse lying level at height `z`: a rim, a cushion's top, a bowl's contents.
    pub fn disc(&mut self, centre: (f32, f32), radius: f32, z: f32, color: formiga_art::Rgba) {
        let (cx, cy) = self.at(centre.0, centre.1, z);
        let rx = radius.round() as i32;
        let ry = (radius / 2.0).round() as i32;
        paint::ellipse(
            &mut self.canvas,
            cx.round() as i32,
            cy.round() as i32,
            rx,
            ry,
            color,
        );
    }

    /// The finished drawing, every edge outlined in a darker shade of its own colour.
    pub fn finish(mut self) -> (Canvas, (i32, i32)) {
        paint::outline_inside(&mut self.canvas, |color| {
            if color.a < 200 {
                color
            } else {
                paint::darker(color, 0.55)
            }
        });
        (self.canvas, self.anchor)
    }
}

/// Every piece's drawing at every turn, made the first time it is asked for.
#[derive(Default)]
pub struct PieceCache {
    sprites: HashMap<(&'static str, u8), Sprite>,
}

impl PieceCache {
    pub fn get(&mut self, piece: &'static Piece, turn: u8) -> &Sprite {
        self.sprites.entry((piece.id, turn % 4)).or_insert_with(|| {
            let away = turn % 4 >= 2;
            let sprite = furniture::draw(piece, away);
            if turn % 2 == 1 {
                sprite.mirrored()
            } else {
                sprite
            }
        })
    }
}

/// The ramps every room shares.
pub mod ramps {
    use crate::paint::Ramp;

    pub const OAK: Ramp = Ramp::new(0x6b3f24, 0x94603a, 0xb67d4c, 0xd29e69, 0xe9c18f);
    pub const WALNUT: Ramp = Ramp::new(0x45281b, 0x6a412d, 0x88583d, 0xa47353, 0xc1916b);
    pub const TERRACOTTA: Ramp = Ramp::new(0x74382a, 0xa5533c, 0xc8714e, 0xe0946c, 0xf2b98e);
    pub const SAGE: Ramp = Ramp::new(0x4a6142, 0x6f8a63, 0x8fae7e, 0xb0c99d, 0xcfe0bd);
    pub const CORNFLOWER: Ramp = Ramp::new(0x46597f, 0x6a80a8, 0x8aa3c9, 0xaec2e0, 0xd0def0);
    pub const ROSE: Ramp = Ramp::new(0x85434f, 0xb5687a, 0xd98c9c, 0xecb0bb, 0xf8d2d8);
    pub const LINEN: Ramp = Ramp::new(0x8c7c62, 0xc9b994, 0xe6d9b8, 0xf4ecd6, 0xfffaf0);
    pub const WICKER: Ramp = Ramp::new(0x775428, 0xa77d42, 0xc9a063, 0xe0bf86, 0xf0d9a8);
    pub const BRASS: Ramp = Ramp::new(0x6a4e20, 0x9c7a36, 0xc7a14d, 0xe2c477, 0xf6e3a8);
    pub const CLAY: Ramp = Ramp::new(0x763823, 0xa9563a, 0xc9744f, 0xe39672, 0xf2b997);
    pub const FERN: Ramp = Ramp::new(0x2c5530, 0x3f7a42, 0x569a52, 0x78b86a, 0xa5d48e);
    pub const BERRY: Ramp = Ramp::new(0x7a2630, 0xa83a47, 0xc8505e, 0xe07c86, 0xf4b2b6);
    pub const GLAZE: Ramp = Ramp::new(0x2d4675, 0x41639e, 0x5b80bf, 0x84a5d8, 0xb8cdee);
    pub const VELVET: Ramp = Ramp::new(0x3e1620, 0x5c2030, 0x7a2c3a, 0x96404c, 0xb45c64);
}
