//! A room's shell: the floor in its finish, the two far walls in theirs, and the dollhouse's cut
//! edges where the near walls and the floor have been sliced away to look in.
//!
//! Every pixel of the floor and walls is worked out from where it is on them, so planks run true,
//! checks meet at their corners and stripes stand straight however big the room is. Light comes
//! from the upper left: the right-hand wall faces it and the left-hand wall is in its own shade,
//! and the floor is a touch darker where it meets the walls.

use crate::iso::{SCENE_HEIGHT, SCENE_WIDTH, SLAB, View, WALL_HEIGHT};
use crate::paint::{self, Ramp, rgb};
use formiga_art::{Canvas, Rgba};

/// How thick the walls are where they are cut, in tiles.
const WALL_THICKNESS: f32 = 0.16;

/// The room's backdrop, floor and walls, ready for everything else to be drawn over.
pub fn draw(view: &View, floor: &str, wall: &str) -> Canvas {
    let mut canvas = Canvas::new(SCENE_WIDTH, SCENE_HEIGHT);
    backdrop(&mut canvas);
    walls(&mut canvas, view, wall);
    floor_tiles(&mut canvas, view, floor);
    slab(&mut canvas, view, floor);
    canvas
}

/// A soft, warm table-top light behind the dollhouse: lighter above, settling below.
fn backdrop(canvas: &mut Canvas) {
    let (top, bottom) = (rgb(0xe8dccb), rgb(0xcdbca5));
    for y in 0..SCENE_HEIGHT as i32 {
        let along = y as f32 / SCENE_HEIGHT as f32;
        for x in 0..SCENE_WIDTH as i32 {
            let across = (x as f32 / SCENE_WIDTH as f32 - 0.5).abs();
            let tone = paint::mix(top, bottom, along * 0.85 + across * 0.25);
            let tone = if paint::chance(x, y, 7, 10) {
                paint::mix(tone, bottom, 0.2)
            } else {
                tone
            };
            canvas.set(x, y, tone);
        }
    }
}

/// The colours a floor finish is drawn in.
fn floor_ramp(floor: &str) -> Ramp {
    match floor {
        "floor.checks" => Ramp::new(0x6f8063, 0x93a684, 0xb4c6a2, 0xe9e2c8, 0xf6f0dc),
        "floor.straw" => Ramp::new(0x8a6a34, 0xb08b4c, 0xcfab68, 0xe2c588, 0xf0dcaa),
        "floor.rose" => Ramp::new(0x8a4f56, 0xb06e74, 0xc98a8a, 0xdcaaa6, 0xecc8c2),
        _ => Ramp::new(0x7a4a2a, 0xa86a3c, 0xc98d52, 0xe0ad72, 0xf2cf98),
    }
}

/// The floor tone at a point on the floor, before its light.
fn floor_tone(floor: &str, ramp: Ramp, fx: f32, fy: f32, px: i32, py: i32) -> Rgba {
    match floor {
        "floor.checks" => {
            let (tx, ty) = (fx.floor() as i32, fy.floor() as i32);
            let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
            if ux < 0.04 || uy < 0.04 {
                return ramp.shadow;
            }
            let pale = (tx + ty) % 2 == 0;
            let tone = if pale { ramp.light } else { ramp.base };
            if paint::chance(px, py, 31, 14) {
                paint::mix(tone, ramp.shadow, 0.25)
            } else {
                tone
            }
        }
        "floor.straw" => {
            // Mats a tile square, their weave turned at every other one, bound at the edges.
            let (tx, ty) = (fx.floor() as i32, fy.floor() as i32);
            let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
            if ux < 0.06 || uy < 0.06 || ux > 0.94 || uy > 0.94 {
                return ramp.shadow;
            }
            let along = if (tx + ty) % 2 == 0 { ux } else { uy };
            let strand = (along * 10.0).floor() as i32;
            let tone = if strand % 2 == 0 {
                ramp.base
            } else {
                ramp.light
            };
            if paint::chance(px, py, 43, 30) {
                paint::mix(tone, ramp.shadow, 0.3)
            } else {
                tone
            }
        }
        "floor.rose" => {
            // A soft pile with a faint lattice woven through it.
            let lattice =
                ((fx + fy) * 2.0).fract() < 0.05 || ((fx - fy) * 2.0).rem_euclid(1.0) < 0.05;
            let tone = if lattice { ramp.light } else { ramp.base };
            match paint::noise(px, py, 53) & 0xff {
                0..=28 => paint::mix(tone, ramp.shadow, 0.35),
                29..=44 => paint::mix(tone, ramp.light, 0.4),
                _ => tone,
            }
        }
        _ => {
            // Boards three to a tile, running along the room's width, their ends staggered.
            let plank = (fy * 3.0).floor() as i32;
            let across = (fy * 3.0).fract();
            if across < 0.09 {
                return ramp.edge;
            }
            let offset = (paint::noise(plank, 0, 61) % 7) as f32 / 7.0 * 2.0;
            let length = (fx + offset) / 2.2;
            if length.fract() < 0.025 {
                return ramp.shadow;
            }
            let board = paint::noise(plank, length.floor() as i32, 67) % 3;
            let tone = match board {
                0 => ramp.base,
                1 => paint::mix(ramp.base, ramp.light, 0.45),
                _ => paint::mix(ramp.base, ramp.shadow, 0.3),
            };
            if across > 0.86 {
                paint::mix(tone, ramp.shadow, 0.35)
            } else if paint::chance(px, py, 71, 16) {
                paint::mix(tone, ramp.shadow, 0.4)
            } else {
                tone
            }
        }
    }
}

fn floor_tiles(canvas: &mut Canvas, view: &View, floor: &str) {
    let ramp = floor_ramp(floor);
    let corners = view.footprint(0, 0, view.width, view.depth);
    let (w, d) = (f32::from(view.width), f32::from(view.depth));
    paint::fill_polygon(&corners, |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let (fx, fy) = (fx.clamp(0.0, w - 0.001), fy.clamp(0.0, d - 0.001));
        let tone = floor_tone(floor, ramp, fx, fy, px, py);
        // Lit from the upper left: lighter towards the left wall's foot, darker to the right.
        let light = ((fy - fx) / (w + d)) * 0.18;
        let tone = if light > 0.0 {
            paint::mix(tone, ramp.shine, light)
        } else {
            paint::mix(tone, ramp.shadow, -light)
        };
        // A little shade where the floor meets either wall.
        let near_wall = fx.min(fy);
        let tone = if near_wall < 0.18 {
            paint::mix(tone, ramp.edge, 0.28 * (1.0 - near_wall / 0.18))
        } else {
            tone
        };
        canvas.set(px, py, tone);
    });
}

/// The colours a wall finish is drawn in, and the second colour of its pattern.
fn wall_ramp(wall: &str) -> (Ramp, Rgba) {
    match wall {
        "wall.stripes" => (
            Ramp::new(0x6f8a78, 0xa3c2ad, 0xcfe6d4, 0xe8f2e4, 0xf6faf2),
            rgb(0xb8d6c0),
        ),
        "wall.leafy" => (
            Ramp::new(0x5f7450, 0x8ea47c, 0xb9cfa0, 0xcfe0b8, 0xe4eed4),
            rgb(0x7f9a6a),
        ),
        "wall.timber" => (
            Ramp::new(0x6b4128, 0x93603c, 0xb57e52, 0xcd9a6c, 0xe2b98c),
            rgb(0x8a5634),
        ),
        _ => (
            Ramp::new(0x9a8466, 0xd2bd98, 0xf0e2c4, 0xf8eed8, 0xfffaec),
            rgb(0xe2d0ae),
        ),
    }
}

/// A wall tone at `along` tiles from the far corner and `up` pixels above the floor.
fn wall_tone(wall: &str, ramp: Ramp, second: Rgba, along: f32, up: f32, px: i32, py: i32) -> Rgba {
    let top = WALL_HEIGHT as f32;
    if up < 6.0 {
        // The skirting board: oak, its top edge catching the light.
        return if up >= 5.0 {
            rgb(0xe9c18f)
        } else if up < 1.0 {
            rgb(0x6b3f24)
        } else {
            rgb(0xb67d4c)
        };
    }
    if up >= top - 3.0 {
        // A plain moulding along the top.
        return if up >= top - 1.0 {
            ramp.shine
        } else {
            ramp.light
        };
    }
    match wall {
        "wall.stripes" => {
            let band = (along * 4.0).floor() as i32;
            if band % 2 == 0 { ramp.base } else { second }
        }
        "wall.leafy" => {
            // Little leaves in a dropped repeat, two to a tile.
            let (cell_x, cell_y) = ((along * 2.0).floor(), ((up - 6.0) / 12.0).floor());
            let shift = if cell_y as i32 % 2 == 0 { 0.0 } else { 0.5 };
            let (ux, uy) = ((along * 2.0 + shift).fract(), ((up - 6.0) / 12.0).fract());
            let (dx, dy) = ((ux - 0.5) * 9.0, (uy - 0.5) * 12.0);
            let leaf = (dx * dx) / 4.0 + (dy - dx * 0.6).powi(2) / 9.0 < 1.0;
            let stem = dx.abs() < 0.5 && dy > 1.0 && dy < 4.0;
            let _ = cell_x;
            if leaf || stem {
                second
            } else if paint::chance(px, py, 83, 10) {
                ramp.light
            } else {
                ramp.base
            }
        }
        "wall.timber" => {
            let board = ((up - 6.0) / 8.0).floor() as i32;
            if ((up - 6.0) / 8.0).fract() < 0.13 {
                return ramp.edge;
            }
            let joint = (along / 1.7 + (board % 2) as f32 * 0.5).fract() < 0.02;
            if joint {
                ramp.shadow
            } else if paint::chance(px, py, 89, 26) {
                second
            } else {
                ramp.base
            }
        }
        _ => {
            if paint::chance(px, py, 97, 18) {
                second
            } else {
                ramp.base
            }
        }
    }
}

fn walls(canvas: &mut Canvas, view: &View, wall: &str) {
    let (ramp, second) = wall_ramp(wall);
    let (w, d) = (f32::from(view.width), f32::from(view.depth));
    let rise = WALL_HEIGHT as f32;
    for (side, length) in [(Side::North, w), (Side::West, d)] {
        let along_to_screen = |along: f32| match side {
            Side::North => view.screen(along, 0.0),
            Side::West => view.screen(0.0, along),
        };
        let (a, b) = (along_to_screen(0.0), along_to_screen(length));
        let face = [a, b, (b.0, b.1 - rise), (a.0, a.1 - rise)];
        paint::fill_polygon(&face, |px, py| {
            let (sx, sy) = (px as f32 + 0.5, py as f32 + 0.5);
            let along = ((sx - a.0) / (b.0 - a.0) * length).clamp(0.0, length - 0.001);
            let floor_y = a.1 + (b.1 - a.1) * along / length;
            let up = (floor_y - sy).clamp(0.0, rise - 0.001);
            let tone = wall_tone(wall, ramp, second, along, up, px, py);
            // The left wall turns away from the light; both walls darken a little towards the
            // corner where they meet.
            let tone = match side {
                Side::West => paint::mix(tone, rgb(0x5a4a5c), 0.16),
                Side::North => tone,
            };
            let corner = (1.0 - along / 1.2).max(0.0);
            canvas.set(px, py, paint::mix(tone, ramp.edge, corner * 0.18));
        });
        // The corner where the walls meet, a line of shade from floor to top.
        let (cx, cy) = (a.0.round() as i32, a.1.round() as i32);
        paint::vline(
            canvas,
            cx,
            cy - WALL_HEIGHT,
            WALL_HEIGHT,
            paint::faded(ramp.edge, 120),
        );
        cut_end(canvas, view, side, length, ramp);
    }
    top_cap(canvas, view, ramp);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    North,
    West,
}

/// The cut end of a wall at the front of the room: its thickness, seen in section.
fn cut_end(canvas: &mut Canvas, view: &View, side: Side, length: f32, ramp: Ramp) {
    let rise = WALL_HEIGHT as f32;
    let (inner, outer) = match side {
        Side::North => (
            view.screen(length, 0.0),
            view.screen(length, -WALL_THICKNESS),
        ),
        Side::West => (
            view.screen(0.0, length),
            view.screen(-WALL_THICKNESS, length),
        ),
    };
    let section = [
        (inner.0, inner.1 + SLAB as f32),
        (outer.0, outer.1 + SLAB as f32),
        (outer.0, outer.1 - rise),
        (inner.0, inner.1 - rise),
    ];
    let core = rgb(0xa58a72);
    paint::fill_polygon(&section, |px, py| {
        let tone = if paint::chance(px, py, 101, 30) {
            paint::mix(core, ramp.edge, 0.3)
        } else {
            core
        };
        canvas.set(px, py, tone);
    });
    let (ix, iy) = (inner.0.round() as i32, inner.1.round() as i32);
    paint::vline(canvas, ix, iy - WALL_HEIGHT, WALL_HEIGHT + SLAB, ramp.edge);
}

/// The top of both walls, seen from above: a strip as thick as the walls.
fn top_cap(canvas: &mut Canvas, view: &View, ramp: Ramp) {
    let rise = WALL_HEIGHT as f32;
    let (w, d) = (f32::from(view.width), f32::from(view.depth));
    let lift = |(x, y): (f32, f32)| (x, y - rise);
    let t = WALL_THICKNESS;
    let north = [
        lift(view.screen(0.0, 0.0)),
        lift(view.screen(w, 0.0)),
        lift(view.screen(w, -t)),
        lift(view.screen(-t, -t)),
    ];
    let west = [
        lift(view.screen(0.0, 0.0)),
        lift(view.screen(-t, -t)),
        lift(view.screen(-t, d)),
        lift(view.screen(0.0, d)),
    ];
    let cap = paint::mix(ramp.light, rgb(0xfff6e4), 0.4);
    for strip in [north, west] {
        paint::polygon(canvas, &strip, cap);
    }
    // The outer edge of the cap, so the walls end crisply against the backdrop.
    let outer = [
        lift(view.screen(-t, d)),
        lift(view.screen(-t, -t)),
        lift(view.screen(w, -t)),
    ];
    for pair in outer.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        paint::line(
            canvas,
            (a.0.round() as i32, a.1.round() as i32),
            (b.0.round() as i32, b.1.round() as i32),
            ramp.edge,
        );
    }
}

/// The floor's front edges in section: the dollhouse's floorboards and the joists under them.
fn slab(canvas: &mut Canvas, view: &View, floor: &str) {
    let ramp = floor_ramp(floor);
    let (w, d) = (f32::from(view.width), f32::from(view.depth));
    let drop = SLAB as f32;
    let left = [
        view.screen(0.0, d),
        view.screen(w, d),
        (view.screen(w, d).0, view.screen(w, d).1 + drop),
        (view.screen(0.0, d).0, view.screen(0.0, d).1 + drop),
    ];
    let right = [
        view.screen(w, 0.0),
        view.screen(w, d),
        (view.screen(w, d).0, view.screen(w, d).1 + drop),
        (view.screen(w, 0.0).0, view.screen(w, 0.0).1 + drop),
    ];
    let wood = Ramp::new(0x5a3520, 0x7c4c30, 0x9a6440, 0xb47e56, 0xd09c72);
    for (face, tone) in [(left, wood.base), (right, wood.shadow)] {
        paint::fill_polygon(&face, |px, py| {
            let fleck = paint::chance(px, py, 107, 24);
            canvas.set(
                px,
                py,
                if fleck {
                    paint::mix(tone, wood.edge, 0.3)
                } else {
                    tone
                },
            );
        });
    }
    // The floor's lit lip, and the edge under it all.
    let lip = |canvas: &mut Canvas, from: (f32, f32), to: (f32, f32), color: Rgba, dy: i32| {
        paint::line(
            canvas,
            (from.0.round() as i32, from.1.round() as i32 + dy),
            (to.0.round() as i32, to.1.round() as i32 + dy),
            color,
        );
    };
    lip(
        canvas,
        view.screen(0.0, d),
        view.screen(w, d),
        ramp.shine,
        0,
    );
    lip(
        canvas,
        view.screen(w, d),
        view.screen(w, 0.0),
        ramp.light,
        0,
    );
    lip(
        canvas,
        view.screen(0.0, d),
        view.screen(w, d),
        wood.edge,
        SLAB,
    );
    lip(
        canvas,
        view.screen(w, d),
        view.screen(w, 0.0),
        wood.edge,
        SLAB,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{FLOORS, WALLS};

    #[test]
    fn every_finish_draws_a_floor_and_walls_with_no_gaps() {
        let view = View::new(8, 8);
        for floor in &FLOORS {
            for wall in &WALLS {
                let canvas = draw(&view, floor.id, wall.id);
                // Every pixel is painted: the backdrop leaves nothing clear.
                assert!(canvas.pixels().iter().all(|pixel| pixel.a == 255));
                // The middle of the floor is the floor, not the backdrop.
                let (cx, cy) = view.tile_centre(4, 4);
                assert_ne!(
                    canvas.get(cx as i32, cy as i32),
                    draw_backdrop_only().get(cx as i32, cy as i32)
                );
            }
        }
    }

    fn draw_backdrop_only() -> Canvas {
        let mut canvas = Canvas::new(SCENE_WIDTH, SCENE_HEIGHT);
        backdrop(&mut canvas);
        canvas
    }
}
