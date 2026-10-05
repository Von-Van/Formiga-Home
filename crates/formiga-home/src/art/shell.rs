//! The house's shell: each room's floor in its finish, the far walls in theirs, and the
//! dollhouse's cut edges where the near walls and the floor have been sliced away to look in.
//!
//! Every pixel of a floor and a wall is worked out from where it is on them, so planks run true,
//! checks meet at their corners and stripes stand straight however big the room is. Light comes
//! from the upper left: the right-hand walls face it and the left-hand walls are in their own
//! shade, and a floor is a touch darker where it meets its walls.
//!
//! Only the walls that stand at full height are part of the shell. A wall cut down low, between
//! two rooms, stands in front of whatever is in the room behind it, so it is drawn with the
//! furniture, in turn, by [`low_wall`].

use crate::house::{Height, House, Room, Wall};
use crate::iso::{SLAB, View, WALL_HEIGHT};
use crate::paint::{self, Ramp, rgb};
use formiga_art::{Canvas, Rgba};
use formiga_home_contract::WallSide;

/// How thick the walls are where they are cut, in tiles.
const WALL_THICKNESS: f32 = 0.16;
/// How tall a wall cut down low stands: its skirting, and a little of the wall above.
pub const LOW_WALL: i32 = 10;

/// The house's floors, its full-height walls and its cut edges, ready for everything else to be
/// drawn over. With `backdrop`, the soft light of a table top behind it all, as a photo has it;
/// without, the picture is clear round the house, for the page it is drawn on.
pub fn draw(view: &View, house: &House, backdrop: bool) -> Canvas {
    let mut canvas = Canvas::new(view.size.0, view.size.1);
    if backdrop {
        paint_backdrop(&mut canvas);
    }
    for room in &house.rooms {
        floor_tiles(&mut canvas, view, room);
    }
    for wall in house.walls.iter().filter(|wall| wall.door) {
        let room = &house.rooms[usize::from(wall.room)];
        match wall.beyond {
            Some(_) => threshold(&mut canvas, view, wall),
            None => mat(&mut canvas, view, wall, room),
        }
    }
    for wall in house.walls.iter().filter(|w| w.height == Height::Full) {
        let room = &house.rooms[usize::from(wall.room)];
        wall_face(&mut canvas, view, wall, room, WALL_HEIGHT);
        if wall.door {
            front_door(&mut canvas, view, wall, false);
        }
    }
    for room in &house.rooms {
        corner_shade(&mut canvas, view, house, room);
    }
    for wall in house.walls.iter().filter(|w| w.height == Height::Full) {
        let room = &house.rooms[usize::from(wall.room)];
        cap(&mut canvas, view, house, wall, room, WALL_HEIGHT);
        if !continues(house, wall, Height::Full) {
            cut_end(&mut canvas, view, house, wall, room, WALL_HEIGHT);
        }
    }
    slab(&mut canvas, view, house);
    canvas
}

/// A soft, warm table-top light behind the dollhouse: lighter above, settling below.
fn paint_backdrop(canvas: &mut Canvas) {
    let (top, bottom) = (rgb(0xe8dccb), rgb(0xcdbca5));
    let (width, height) = (canvas.width() as i32, canvas.height() as i32);
    for y in 0..height {
        let along = y as f32 / height as f32;
        for x in 0..width {
            let across = (x as f32 / width as f32 - 0.5).abs();
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
        "floor.sand" => Ramp::new(0x7e7668, 0xa49c8c, 0xc4bcaa, 0xdcd6c6, 0xeeeade),
        "floor.moss" => Ramp::new(0x3a5426, 0x52722f, 0x6a8f3c, 0x88ab52, 0xaecb78),
        "floor.night" => Ramp::new(0x161c3c, 0x242d5c, 0x34407c, 0x4c5c9a, 0x6c7cba),
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
        "floor.sand" => {
            // Wide boards washed pale by the sea, two to a tile, their grain in long streaks.
            let plank = (fy * 2.0).floor() as i32;
            let across = (fy * 2.0).fract();
            if across < 0.06 {
                return ramp.shadow;
            }
            let offset = (paint::noise(plank, 0, 141) % 5) as f32 / 5.0 * 3.0;
            let length = (fx + offset) / 3.0;
            if length.fract() < 0.02 {
                return ramp.shadow;
            }
            let streak = paint::noise((fx * 6.0) as i32, (fy * 40.0) as i32, 143).is_multiple_of(9);
            if streak {
                paint::mix(ramp.base, ramp.shadow, 0.45)
            } else if across > 0.9 {
                paint::mix(ramp.base, ramp.shadow, 0.25)
            } else if paint::chance(px, py, 147, 20) {
                ramp.light
            } else {
                ramp.base
            }
        }
        "floor.moss" => {
            // Soft moss in clumps, lit on their upper sides, with now and then a tiny flower.
            let clump = paint::noise(px.div_euclid(2), py.div_euclid(2), 151) % 100;
            if paint::chance(px, py, 153, 3) {
                return if paint::noise(px, py, 155).is_multiple_of(2) {
                    rgb(0xfff4c8)
                } else {
                    rgb(0xf0b8c4)
                };
            }
            match clump {
                0..=16 => ramp.light,
                17..=22 => ramp.shine,
                23..=38 => ramp.shadow,
                _ => ramp.base,
            }
        }
        "floor.night" => {
            // Tiles in the blues of the night, with a gold stud where four meet.
            let (tx, ty) = (fx.floor() as i32, fy.floor() as i32);
            let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
            if !(0.06..=0.94).contains(&ux) && !(0.06..=0.94).contains(&uy) {
                return rgb(0xe8c25a);
            }
            if ux < 0.03 || uy < 0.03 {
                return ramp.edge;
            }
            let tone = if (tx + ty) % 2 == 0 {
                ramp.base
            } else {
                ramp.shadow
            };
            if paint::chance(px, py, 157, 5) {
                ramp.shine
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

fn floor_tiles(canvas: &mut Canvas, view: &View, room: &Room) {
    let finish = room.floor.as_str();
    let ramp = floor_ramp(finish);
    let corners = view.footprint(room.x, room.y, room.width, room.depth);
    let (w, d) = (f32::from(room.width), f32::from(room.depth));
    let (left, top) = (f32::from(room.x), f32::from(room.y));
    paint::fill_polygon(&corners, |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        // The pattern starts at the room's own far corner.
        let (fx, fy) = (
            (fx - left).clamp(0.0, w - 0.001),
            (fy - top).clamp(0.0, d - 0.001),
        );
        let tone = floor_tone(finish, ramp, fx, fy, px, py);
        // Lit from the upper left: lighter towards the left wall's foot, darker to the right.
        let light = ((fy - fx) / (w + d)) * 0.18;
        let tone = if light > 0.0 {
            paint::mix(tone, ramp.shine, light)
        } else {
            paint::mix(tone, ramp.shadow, -light)
        };
        // A little shade where the floor meets either far wall.
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
        "wall.waves" => (
            Ramp::new(0x2c5a7a, 0x4c82a6, 0x7eaccc, 0xb8d4e6, 0xe2eef6),
            rgb(0xeef6f8),
        ),
        "wall.birch" => (
            Ramp::new(0x6a645a, 0xb8b2a6, 0xe4e0d6, 0xf2efe8, 0xfdfcf8),
            rgb(0x4a443c),
        ),
        "wall.stars" => (
            Ramp::new(0x10163a, 0x1c2554, 0x2a3570, 0x3e4c8e, 0x5c6cae),
            rgb(0xe8c25a),
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
        "wall.waves" => {
            // Bands of sea and foam, rolling along the wall.
            let wave = (up - 6.0) / 7.0 + (along * std::f32::consts::TAU).sin() * 0.3;
            if (wave.floor() as i32) % 2 == 0 {
                ramp.base
            } else if paint::chance(px, py, 161, 12) {
                ramp.light
            } else {
                second
            }
        }
        "wall.birch" => {
            // Birch panels, half a tile wide, with the dark marks birch bark has.
            if (along * 2.0).fract() < 0.05 {
                return ramp.shadow;
            }
            let mark =
                paint::noise((along * 5.0) as i32, (up / 1.5) as i32, 163).is_multiple_of(17);
            if mark {
                second
            } else if paint::chance(px, py, 167, 16) {
                ramp.light
            } else {
                ramp.base
            }
        }
        "wall.stars" => {
            // Little gold stars in a dropped repeat on the blue of the night.
            let row = ((up - 6.0) / 10.0).floor();
            let shift = if row as i32 % 2 == 0 { 0.0 } else { 0.5 };
            let (ux, uy) = ((along * 2.0 + shift).fract(), ((up - 6.0) / 10.0).fract());
            let (dx, dy) = (((ux - 0.5) * 16.0).abs(), ((uy - 0.5) * 10.0).abs());
            if (dx < 0.6 && dy < 2.0) || (dy < 0.6 && dx < 2.0) {
                second
            } else if paint::chance(px, py, 169, 6) {
                ramp.shine
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

/// A point `up` pixels above a spot on the floor, on screen.
fn raised(view: &View, at: (f32, f32), up: i32) -> (f32, f32) {
    let (x, y) = view.screen(at.0, at.1);
    (x, y - up as f32)
}

/// How far one along a wall's length is, on the floor.
fn along_wall(side: WallSide) -> (f32, f32) {
    match side {
        WallSide::North => (1.0, 0.0),
        WallSide::West => (0.0, 1.0),
    }
}

/// How far out through a wall is, on the floor: the way its thickness goes, away from its room.
fn through_wall(side: WallSide) -> (f32, f32) {
    match side {
        WallSide::North => (0.0, -WALL_THICKNESS),
        WallSide::West => (-WALL_THICKNESS, 0.0),
    }
}

/// One tile's length of wall, `rise` pixels tall, in its room's finish.
fn wall_face(canvas: &mut Canvas, view: &View, wall: &Wall, room: &Room, rise: i32) {
    let finish = room.wall.as_str();
    let (ramp, second) = wall_ramp(finish);
    let (a, b) = wall.foot();
    let (a, b) = (view.screen(a.0, a.1), view.screen(b.0, b.1));
    let top = rise as f32;
    let face = [a, b, (b.0, b.1 - top), (a.0, a.1 - top)];
    paint::fill_polygon(&face, |px, py| {
        let (sx, sy) = (px as f32 + 0.5, py as f32 + 0.5);
        let across = ((sx - a.0) / (b.0 - a.0)).clamp(0.0, 0.999);
        let along = f32::from(wall.at) + across;
        let floor_y = a.1 + (b.1 - a.1) * across;
        let up = (floor_y - sy).clamp(0.0, top - 0.001);
        let tone = if rise < WALL_HEIGHT && up >= top - 1.0 {
            // A wall cut down shows the line it was cut along.
            ramp.light
        } else {
            wall_tone(finish, ramp, second, along, up, px, py)
        };
        // The left-hand walls turn away from the light; every wall darkens a little towards its
        // room's far corner.
        let tone = match wall.side {
            WallSide::West => paint::mix(tone, rgb(0x5a4a5c), 0.16),
            WallSide::North => tone,
        };
        let corner = (1.0 - along / 1.2).max(0.0);
        canvas.set(px, py, paint::mix(tone, ramp.edge, corner * 0.18));
    });
}

/// The line of shade up the corner where a room's two far walls meet, if both stand there.
fn corner_shade(canvas: &mut Canvas, view: &View, house: &House, room: &Room) {
    let Some(index) = house.rooms.iter().position(|other| other == room) else {
        return;
    };
    let full = |side| {
        house
            .wall(index as u8, side, 0)
            .is_some_and(|wall| wall.height == Height::Full)
    };
    if !(full(WallSide::North) && full(WallSide::West)) {
        return;
    }
    let (ramp, _) = wall_ramp(room.wall.as_str());
    let (cx, cy) = view.pixel(f32::from(room.x), f32::from(room.y));
    paint::vline(
        canvas,
        cx,
        cy - WALL_HEIGHT,
        WALL_HEIGHT,
        paint::faded(ramp.edge, 120),
    );
}

/// Whether the wall goes on past this stretch at the same height: into the next stretch along it,
/// in this room or the next.
fn continues(house: &House, wall: &Wall, height: Height) -> bool {
    let (dx, dy) = along_wall(wall.side);
    let next = (wall.x + dx as u8, wall.y + dy as u8);
    house.walls.iter().any(|other| {
        other.side == wall.side
            && (other.x, other.y) == next
            && other.height == height
            && !(height == Height::Low && other.door)
    })
}

/// The top of a stretch of wall, seen from above: a strip as thick as the wall, with its outer
/// edge drawn so the wall ends crisply against whatever is behind it. Where a room's two far
/// walls meet, the corner is filled.
fn cap(canvas: &mut Canvas, view: &View, house: &House, wall: &Wall, room: &Room, rise: i32) {
    let (ramp, _) = wall_ramp(room.wall.as_str());
    let (a, b) = wall.foot();
    let (tx, ty) = through_wall(wall.side);
    // At the room's far corner, a north wall's cap reaches over the west wall's too.
    let corner = wall.at == 0
        && wall.side == WallSide::North
        && house
            .wall(wall.room, WallSide::West, 0)
            .is_some_and(|west| west.height == Height::Full || rise < WALL_HEIGHT);
    let back = if corner {
        (a.0 - WALL_THICKNESS, a.1 + ty)
    } else {
        (a.0 + tx, a.1 + ty)
    };
    let strip = [
        raised(view, a, rise),
        raised(view, b, rise),
        raised(view, (b.0 + tx, b.1 + ty), rise),
        raised(view, back, rise),
    ];
    let cap = paint::mix(ramp.light, rgb(0xfff6e4), 0.4);
    paint::polygon(canvas, &strip, cap);
    let px = |p: (f32, f32)| (p.0.round() as i32, p.1.round() as i32);
    paint::line(canvas, px(strip[3]), px(strip[2]), ramp.edge);
    if corner {
        let west_back = raised(view, (a.0 - WALL_THICKNESS, a.1 + 0.001), rise);
        paint::line(canvas, px(strip[3]), px(west_back), ramp.edge);
    }
}

/// The cut end of a stretch of wall where it stops: its thickness, seen in section. At the
/// house's near edge the section runs on down through the floor's thickness.
fn cut_end(canvas: &mut Canvas, view: &View, house: &House, wall: &Wall, room: &Room, rise: i32) {
    let (ramp, _) = wall_ramp(room.wall.as_str());
    let (_, b) = wall.foot();
    let (tx, ty) = through_wall(wall.side);
    // Past the end, is there floor? If not, this is the house's edge, cut through.
    let past = match wall.side {
        WallSide::North => (wall.x as i32 + 1, wall.y as i32),
        WallSide::West => (wall.x as i32, wall.y as i32 + 1),
    };
    let drop = if house.room_at(past.0, past.1).is_none() {
        SLAB as f32
    } else {
        0.0
    };
    let inner = view.screen(b.0, b.1);
    let outer = view.screen(b.0 + tx, b.1 + ty);
    let top = rise as f32;
    let section = [
        (inner.0, inner.1 + drop),
        (outer.0, outer.1 + drop),
        (outer.0, outer.1 - top),
        (inner.0, inner.1 - top),
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
    paint::vline(canvas, ix, iy - rise, rise + drop as i32, ramp.edge);
}

/// A wall cut down low, a tile of it, drawn in turn with the furniture: in front of whatever is
/// in the room behind it, and behind whatever is in its own. Nothing is drawn for a doorway.
pub fn low_wall(canvas: &mut Canvas, view: &View, house: &House, wall: &Wall) {
    if wall.door {
        return;
    }
    let room = &house.rooms[usize::from(wall.room)];
    wall_face(canvas, view, wall, room, LOW_WALL);
    cap(canvas, view, house, wall, room, LOW_WALL);
    let next_full = continues(house, wall, Height::Full);
    if !continues(house, wall, Height::Low) && !next_full {
        cut_end(canvas, view, house, wall, room, LOW_WALL);
    }
}

/// Where a low wall's picture falls on screen, inclusive, for working out what it stands in
/// front of.
pub fn low_wall_rect(view: &View, wall: &Wall) -> (i32, i32, i32, i32) {
    let (a, b) = wall.foot();
    let (tx, ty) = through_wall(wall.side);
    let points = [
        view.screen(a.0, a.1),
        view.screen(b.0, b.1),
        view.screen(a.0 + tx, a.1 + ty),
        view.screen(b.0 + tx, b.1 + ty),
    ];
    let left = points.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32;
    let right = points.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32;
    let top = points.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32 - LOW_WALL;
    let bottom = points.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32 + SLAB;
    (left, top, right, bottom)
}

/// The sill across a doorway between two rooms: a strip of oak where the wall would be.
fn threshold(canvas: &mut Canvas, view: &View, wall: &Wall) {
    let (a, b) = wall.foot();
    let (tx, ty) = through_wall(wall.side);
    let (ox, oy) = (-tx * 0.4, -ty * 0.4);
    let strip = [
        view.screen(a.0 + tx, a.1 + ty),
        view.screen(b.0 + tx, b.1 + ty),
        view.screen(b.0 + ox, b.1 + oy),
        view.screen(a.0 + ox, a.1 + oy),
    ];
    let oak = Ramp::new(0x6b3f24, 0x9a6440, 0xb67d4c, 0xd09c6a, 0xe9c18f);
    paint::fill_polygon(&strip, |px, py| {
        let tone = if paint::chance(px, py, 113, 40) {
            oak.shadow
        } else {
            oak.light
        };
        canvas.set(px, py, tone);
    });
    let px = |p: (f32, f32)| (p.0.round() as i32, p.1.round() as i32);
    paint::line(canvas, px(strip[3]), px(strip[2]), oak.edge);
}

/// A doormat inside the front door.
fn mat(canvas: &mut Canvas, view: &View, wall: &Wall, room: &Room) {
    let (a, _) = wall.foot();
    let (dx, dy) = along_wall(wall.side);
    let (ix, iy) = match wall.side {
        WallSide::North => (0.0, 1.0),
        WallSide::West => (1.0, 0.0),
    };
    let corner = |along: f32, inward: f32| {
        view.screen(
            a.0 + dx * along + ix * inward,
            a.1 + dy * along + iy * inward,
        )
    };
    let shape = [
        corner(0.18, 0.08),
        corner(0.82, 0.08),
        corner(0.82, 0.5),
        corner(0.18, 0.5),
    ];
    let coir = Ramp::new(0x6a4a2a, 0x9a7444, 0xbf9658, 0xd8b474, 0xead096);
    let floor = floor_ramp(room.floor.as_str());
    paint::fill_polygon(&shape, |px, py| {
        let tone = if paint::chance(px, py, 117, 70) {
            coir.shadow
        } else {
            coir.base
        };
        canvas.set(px, py, tone);
    });
    let px = |p: (f32, f32)| (p.0.round() as i32, p.1.round() as i32);
    paint::line(canvas, px(shape[3]), px(shape[2]), floor.edge);
    paint::line(canvas, px(shape[1]), px(shape[2]), coir.edge);
}

/// The front door, in a full-height wall: shut, or standing open while someone comes or goes,
/// with a glimpse of the garden through it.
pub fn front_door(canvas: &mut Canvas, view: &View, wall: &Wall, open: bool) {
    const TALL: f32 = 38.0;
    let (a, b) = wall.foot();
    let (a, b) = (view.screen(a.0, a.1), view.screen(b.0, b.1));
    let face = [a, b, (b.0, b.1 - TALL - 3.0), (a.0, a.1 - TALL - 3.0)];
    let oak = Ramp::new(0x4a2a1c, 0x6e4128, 0x8f5a36, 0xb07a4c, 0xcf9c6a);
    let paint_ = Ramp::new(0x2f4a3c, 0x46705a, 0x5c8f74, 0x7aae92, 0xa2cdb4);
    let garden = (rgb(0xcfe6f0), rgb(0x8cc178));
    paint::fill_polygon(&face, |px, py| {
        let (sx, sy) = (px as f32 + 0.5, py as f32 + 0.5);
        let across = ((sx - a.0) / (b.0 - a.0)).clamp(0.0, 0.999);
        let floor_y = a.1 + (b.1 - a.1) * across;
        let up = floor_y - sy;
        if !(0.0..TALL + 3.0).contains(&up) || !(0.1..0.9).contains(&across) {
            return;
        }
        let frame = !(0.18..0.82).contains(&across) || up >= TALL;
        let tone = if frame {
            if up >= TALL + 2.0 || across < 0.12 {
                oak.light
            } else {
                oak.base
            }
        } else if open {
            // The garden outside, the sky above the hedge, and the door swung back on the left.
            if across < 0.34 {
                if across > 0.31 {
                    paint_.edge
                } else {
                    paint_.shadow
                }
            } else if up > TALL * 0.55 {
                garden.0
            } else if paint::chance(px, py, 127, 60) {
                paint::mix(garden.1, rgb(0x5f8f4e), 0.5)
            } else {
                garden.1
            }
        } else {
            // Painted boards with a little window and a brass knob.
            let window = (0.32..0.68).contains(&across) && (TALL * 0.62..TALL * 0.86).contains(&up);
            let knob = (0.66..0.74).contains(&across) && (TALL * 0.4..TALL * 0.48).contains(&up);
            if knob {
                rgb(0xe8c25a)
            } else if window {
                if up > TALL * 0.8 {
                    rgb(0xe9f6f7)
                } else {
                    rgb(0xa9d4e0)
                }
            } else if (across * 6.0).fract() < 0.12 {
                paint_.shadow
            } else if across < 0.26 {
                paint_.light
            } else {
                paint_.base
            }
        };
        let tone = match wall.side {
            WallSide::West => paint::mix(tone, rgb(0x5a4a5c), 0.14),
            WallSide::North => tone,
        };
        canvas.set(px, py, tone);
    });
}

/// A little of a floor finish, two tiles a side, for choosing it by.
pub fn floor_swatch(finish: &'static str) -> Canvas {
    let view = View {
        width: 2,
        depth: 2,
        origin: (32.0, 0.0),
        size: (64, 32 + SLAB as u32),
    };
    let room = Room {
        x: 0,
        y: 0,
        width: 2,
        depth: 2,
        floor: formiga_home_contract::CatalogId::known(finish),
        wall: formiga_home_contract::CatalogId::known("wall.plaster"),
        kind: None,
    };
    let mut canvas = Canvas::new(view.size.0, view.size.1);
    floor_tiles(&mut canvas, &view, &room);
    let house = House::of(&[formiga_home_contract::RoomLayout {
        width: 2,
        depth: 2,
        floor: room.floor.clone(),
        wall: room.wall.clone(),
        pieces: Vec::new(),
        displays: Vec::new(),
        plan: None,
        kind: None,
        doors: Vec::new(),
    }]);
    slab(&mut canvas, &view, &house);
    canvas
}

/// A little of a wall finish, two tiles along and as tall as a wall, for choosing it by.
pub fn wall_swatch(finish: &'static str) -> Canvas {
    let view = View {
        width: 2,
        depth: 1,
        origin: (0.0, WALL_HEIGHT as f32),
        size: (32, WALL_HEIGHT as u32 + 16),
    };
    let room = Room {
        x: 0,
        y: 0,
        width: 2,
        depth: 1,
        floor: formiga_home_contract::CatalogId::known("floor.boards"),
        wall: formiga_home_contract::CatalogId::known(finish),
        kind: None,
    };
    let mut canvas = Canvas::new(view.size.0, view.size.1);
    for at in 0..2 {
        let wall = Wall {
            room: 0,
            side: WallSide::North,
            at,
            x: at,
            y: 0,
            height: Height::Full,
            door: false,
            beyond: None,
        };
        wall_face(&mut canvas, &view, &wall, &room, WALL_HEIGHT);
    }
    canvas
}

/// The floor's near edges in section: the dollhouse's floorboards and the joists under them,
/// wherever a room's front has no room in front of it.
fn slab(canvas: &mut Canvas, view: &View, house: &House) {
    let wood = Ramp::new(0x5a3520, 0x7c4c30, 0x9a6440, 0xb47e56, 0xd09c72);
    let drop = SLAB as f32;
    let edges = house.cut_edges();
    for &(x, y, left) in &edges {
        let (fx, fy) = (f32::from(x), f32::from(y));
        let (from, to) = if left {
            (view.screen(fx, fy + 1.0), view.screen(fx + 1.0, fy + 1.0))
        } else {
            (view.screen(fx + 1.0, fy), view.screen(fx + 1.0, fy + 1.0))
        };
        let face = [from, to, (to.0, to.1 + drop), (from.0, from.1 + drop)];
        let tone = if left { wood.base } else { wood.shadow };
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
    for &(x, y, left) in &edges {
        let Some(room) = house.room_at(i32::from(x), i32::from(y)) else {
            continue;
        };
        let ramp = floor_ramp(house.rooms[usize::from(room)].floor.as_str());
        let (fx, fy) = (f32::from(x), f32::from(y));
        let (from, to) = if left {
            (view.screen(fx, fy + 1.0), view.screen(fx + 1.0, fy + 1.0))
        } else {
            (view.screen(fx + 1.0, fy), view.screen(fx + 1.0, fy + 1.0))
        };
        let px = |p: (f32, f32), dy: i32| (p.0.round() as i32, p.1.round() as i32 + dy);
        let lip = if left { ramp.shine } else { ramp.light };
        paint::line(canvas, px(from, 0), px(to, 0), lip);
        paint::line(canvas, px(from, SLAB), px(to, SLAB), wood.edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{FLOORS, WALLS};
    use formiga_home_contract::{CatalogId, RoomLayout, sample};

    #[test]
    fn every_finish_draws_a_floor_and_walls_with_no_gaps() {
        let mut layout: RoomLayout = crate::starter::room(&sample::snapshot());
        for floor in &FLOORS {
            for wall in &WALLS {
                layout.floor = CatalogId::known(floor.id);
                layout.wall = CatalogId::known(wall.id);
                let house = House::of(std::slice::from_ref(&layout));
                let view = View::of(&house);
                let canvas = draw(&view, &house, true);
                // Every pixel is painted: the backdrop leaves nothing clear.
                assert!(canvas.pixels().iter().all(|pixel| pixel.a == 255));
                // The middle of the floor is the floor, not the backdrop.
                let (cx, cy) = view.tile_centre(4, 4);
                let mut bare = Canvas::new(view.size.0, view.size.1);
                paint_backdrop(&mut bare);
                assert_ne!(
                    canvas.get(cx as i32, cy as i32),
                    bare.get(cx as i32, cy as i32)
                );
                // Without the backdrop, the picture is clear round the house.
                let clear = draw(&view, &house, false);
                assert_eq!(clear.get(0, 0).a, 0);
                assert_eq!(clear.get(cx as i32, cy as i32).a, 255);
            }
        }
    }
}
