//! Every piece of furniture, drawn on its own footprint from the front (`away` false: its front
//! faces down the room's depth, towards the lower left) or from behind. Each is built from blocks
//! and a few round things, back to front, then edged in a darker shade of its own colours.

use super::ramps::*;
use super::{Block, Easel, Sprite};
use crate::catalog::Piece;
use crate::paint::{self, rgb, rgba};
use formiga_art::Canvas;

/// A piece's drawing, from the front or from behind.
pub fn draw(piece: &Piece, away: bool) -> Sprite {
    let (w, d) = piece.size;
    let mut easel = Easel::new(w, d, piece.height.max(8) + 4, salt(piece.id));
    let mut over: Option<Easel> = None;
    let flip = |block: Block| {
        if away {
            block.facing_away(f32::from(d))
        } else {
            block
        }
    };
    match piece.id {
        "cushion" => cushion(&mut easel),
        "armchair" | "sofa" => over = Some(seat(&mut easel, piece, away)),
        "bed" => bed(&mut easel, flip, away),
        "basket" => basket(&mut easel),
        "side_table" => side_table(&mut easel),
        "low_table" => low_table(&mut easel),
        "shelf" => shelf(&mut easel),
        "case" => over = Some(case(&mut easel, piece)),
        "round_rug" => round_rug(&mut easel),
        "long_rug" => long_rug(&mut easel),
        "lamp" => lamp(&mut easel),
        "fern" => fern(&mut easel),
        "ball" => ball(&mut easel),
        "snack_bowl" => snack_bowl(&mut easel),
        _ => unknown(&mut easel),
    }
    let (canvas, anchor) = easel.finish();
    Sprite {
        canvas,
        anchor,
        over: over.map(|easel| easel.finish().0),
    }
}

fn salt(id: &str) -> u32 {
    id.bytes().fold(2166136261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    })
}

/// Four little feet under a box, set in from its corners.
fn feet(x: (f32, f32), y: (f32, f32), height: f32, inset: f32) -> Vec<Block> {
    let size = 0.1;
    [
        (x.0 + inset, y.0 + inset),
        (x.1 - inset - size, y.0 + inset),
        (x.0 + inset, y.1 - inset - size),
        (x.1 - inset - size, y.1 - inset - size),
    ]
    .into_iter()
    .map(|(fx, fy)| Block::new((fx, fx + size), (fy, fy + size), (0.0, height), WALNUT))
    .collect()
}

fn cushion(easel: &mut Easel) {
    easel.shadow((0.12, 0.88), (0.12, 0.88), 60);
    easel.block(Block::new(
        (0.16, 0.84),
        (0.16, 0.84),
        (0.0, 4.0),
        CORNFLOWER,
    ));
    // Plumped up on top, with a tuft in the middle.
    easel.disc((0.5, 0.5), 9.0, 5.0, CORNFLOWER.light);
    easel.disc((0.46, 0.46), 5.0, 6.0, CORNFLOWER.shine);
    let (cx, cy) = easel.pixel(0.5, 0.5, 6.0);
    paint::put(&mut easel.canvas, cx, cy, CORNFLOWER.shadow);
    paint::put(&mut easel.canvas, cx + 1, cy, CORNFLOWER.shadow);
}

/// A chair or a sofa, in two parts: what is behind whoever sits in it, and what is in front of
/// them — the near arm, and, turned away, the back.
fn seat(easel: &mut Easel, piece: &Piece, away: bool) -> Easel {
    let (w, d) = (f32::from(piece.size.0), f32::from(piece.size.1));
    let (cloth, cushion) = if piece.id == "sofa" {
        (SAGE, LINEN)
    } else {
        (TERRACOTTA, ROSE)
    };
    easel.shadow((0.06, w - 0.06), (0.08, d - 0.06), 70);
    let flip = |block: Block| if away { block.facing_away(d) } else { block };
    let back = flip(Block::new(
        (0.08, w - 0.08),
        (0.08, 0.3),
        (2.0, 28.0),
        cloth,
    ));
    let far_arm = flip(Block::new(
        (0.06, 0.24),
        (0.26, d - 0.06),
        (2.0, 15.0),
        cloth,
    ));
    let near_arm = flip(Block::new(
        (w - 0.24, w - 0.06),
        (0.26, d - 0.06),
        (2.0, 15.0),
        cloth,
    ));
    let mut behind = feet((0.1, w - 0.1), (0.14, d - 0.08), 2.0, 0.02);
    behind.push(flip(Block::new(
        (0.24, w - 0.24),
        (0.28, d - 0.1),
        (2.0, 9.0),
        cloth,
    )));
    let cushions: Vec<(f32, f32)> = if piece.id == "sofa" {
        vec![(0.26, 0.98), (1.02, w - 0.26)]
    } else {
        vec![(0.26, w - 0.26)]
    };
    for x in cushions {
        behind.push(flip(Block::new(x, (0.32, d - 0.14), (9.0, 11.0), cushion)));
    }
    behind.push(far_arm);
    let mut front = Easel::new(
        piece.size.0,
        piece.size.1,
        piece.height.max(8) + 4,
        easel.salt,
    );
    if away {
        easel.blocks(behind);
        front.blocks(vec![back, near_arm]);
    } else {
        behind.push(back);
        easel.blocks(behind);
        front.block(near_arm);
    }
    front
}

fn bed(easel: &mut Easel, flip: impl Fn(Block) -> Block, away: bool) {
    easel.shadow((0.06, 0.94), (0.04, 1.96), 70);
    let mut blocks = feet((0.1, 0.9), (0.1, 1.9), 3.0, 0.0);
    blocks.extend([
        // The headboard at the head, the far end, with the frame, the mattress and the
        // blanket in front of it.
        Block::new((0.06, 0.94), (0.04, 0.16), (2.0, 19.0), WALNUT),
        Block::new((0.1, 0.9), (0.16, 1.9), (3.0, 7.0), OAK),
        Block::new((0.13, 0.87), (0.18, 1.86), (7.0, 10.0), LINEN),
        Block::new((0.13, 0.87), (0.72, 1.88), (10.0, 12.0), ROSE),
        Block::new((0.08, 0.92), (1.84, 1.95), (2.0, 10.0), WALNUT),
    ]);
    easel.blocks(blocks.into_iter().map(&flip).collect());
    // The pillow, plump at the head; and the blanket's turned-down hem.
    let head = if away { 1.62 } else { 0.38 };
    easel.disc((0.5, head), 8.0, 11.5, LINEN.light);
    easel.disc((0.46, head - 0.03), 5.0, 12.5, LINEN.shine);
    let hem = if away { 1.18 } else { 0.76 };
    let (a, b) = (easel.pixel(0.14, hem, 12.0), easel.pixel(0.86, hem, 12.0));
    paint::line(&mut easel.canvas, a, b, ROSE.shine);
}

fn basket(easel: &mut Easel) {
    easel.shadow((0.12, 0.88), (0.12, 0.88), 55);
    let centre = (0.5, 0.5);
    easel.cylinder(centre, (10.0, 11.5), (0.0, 7.0), WICKER);
    // Woven bands round the body.
    let (cx, cy) = easel.pixel(0.5, 0.5, 0.0);
    for row in [2, 5] {
        for dx in -10..=10 {
            if (dx + row) % 3 == 0 {
                paint::put(
                    &mut easel.canvas,
                    cx + dx,
                    cy - row + (dx.abs() / 4),
                    WICKER.shadow,
                );
            }
        }
    }
    easel.disc(centre, 11.5, 7.0, WICKER.light);
    easel.disc(centre, 9.5, 7.0, WICKER.edge);
    easel.disc(centre, 8.5, 5.5, LINEN.base);
    easel.disc((0.46, 0.46), 5.5, 6.0, LINEN.light);
}

fn side_table(easel: &mut Easel) {
    easel.shadow((0.14, 0.86), (0.14, 0.86), 50);
    let mut blocks = vec![];
    for (x, y) in [(0.2, 0.2), (0.7, 0.2), (0.2, 0.7), (0.7, 0.7)] {
        blocks.push(Block::new((x, x + 0.1), (y, y + 0.1), (0.0, 12.0), WALNUT));
    }
    blocks.push(Block::new((0.24, 0.76), (0.24, 0.76), (4.0, 5.0), OAK));
    blocks.push(Block::new((0.14, 0.86), (0.14, 0.86), (12.0, 15.0), OAK));
    easel.blocks(blocks);
}

fn low_table(easel: &mut Easel) {
    easel.shadow((0.08, 1.92), (0.12, 0.88), 50);
    let mut blocks = feet((0.12, 1.88), (0.18, 0.82), 8.0, 0.02);
    blocks.push(Block::new((0.08, 1.92), (0.12, 0.88), (8.0, 11.0), OAK));
    easel.blocks(blocks);
}

/// An open shelf: posts at its three far corners and four boards between them, open at the near
/// corner so nothing on it is ever cut in two.
fn shelf(easel: &mut Easel) {
    easel.shadow((0.1, 0.9), (0.1, 0.9), 60);
    let post = |x: f32, y: f32| Block::new((x, x + 0.11), (y, y + 0.11), (0.0, 44.0), OAK);
    let board = |z: f32| Block::new((0.12, 0.88), (0.12, 0.88), (z - 2.0, z), OAK);
    let mut blocks = vec![post(0.12, 0.12), post(0.77, 0.12), post(0.12, 0.77)];
    blocks.extend([board(4.0), board(18.0), board(32.0), board(44.0)]);
    easel.blocks(blocks);
}

/// A glass case on a plinth, lined in velvet like Formiga Hill's own. Its glass is drawn in front
/// of what is shown in it.
fn case(easel: &mut Easel, piece: &Piece) -> Easel {
    easel.shadow((0.08, 0.92), (0.08, 0.92), 60);
    easel.block(Block::new((0.1, 0.9), (0.1, 0.9), (0.0, 6.0), WALNUT));
    // The velvet lining the two far sides, seen through the glass.
    let back_x = [
        easel.at(0.12, 0.12, 33.0),
        easel.at(0.12, 0.88, 33.0),
        easel.at(0.12, 0.88, 6.0),
        easel.at(0.12, 0.12, 6.0),
    ];
    let back_y = [
        easel.at(0.12, 0.12, 33.0),
        easel.at(0.88, 0.12, 33.0),
        easel.at(0.88, 0.12, 6.0),
        easel.at(0.12, 0.12, 6.0),
    ];
    paint::polygon(&mut easel.canvas, &back_x, VELVET.shadow);
    paint::polygon(&mut easel.canvas, &back_y, VELVET.base);
    for z in [12.0, 25.0] {
        easel.block(Block::new((0.13, 0.87), (0.13, 0.87), (z - 1.0, z), LINEN));
    }
    easel.block(Block::new((0.1, 0.9), (0.1, 0.9), (33.0, 36.0), WALNUT));
    let mut glass = Easel::new(
        piece.size.0,
        piece.size.1,
        piece.height.max(8) + 4,
        easel.salt,
    );
    let pane = rgba(0xd8f0f2, 70);
    let front_left = [
        glass.at(0.1, 0.9, 33.0),
        glass.at(0.9, 0.9, 33.0),
        glass.at(0.9, 0.9, 6.0),
        glass.at(0.1, 0.9, 6.0),
    ];
    let front_right = [
        glass.at(0.9, 0.1, 33.0),
        glass.at(0.9, 0.9, 33.0),
        glass.at(0.9, 0.9, 6.0),
        glass.at(0.9, 0.1, 6.0),
    ];
    paint::polygon(&mut glass.canvas, &front_left, pane);
    paint::polygon(&mut glass.canvas, &front_right, rgba(0xc4e2e6, 80));
    // The panes' meeting edge, a bright line rather than a post, so it never hides what is
    // shown; and a glint across each pane.
    let (top, bottom) = (glass.pixel(0.9, 0.9, 32.0), glass.pixel(0.9, 0.9, 7.0));
    paint::line(&mut glass.canvas, top, bottom, rgba(0xf4fcfc, 120));
    for (from, to) in [
        (glass.pixel(0.25, 0.9, 28.0), glass.pixel(0.45, 0.9, 12.0)),
        (glass.pixel(0.9, 0.3, 29.0), glass.pixel(0.9, 0.42, 20.0)),
    ] {
        paint::line(&mut glass.canvas, from, to, rgba(0xffffff, 150));
    }
    glass
}

fn round_rug(easel: &mut Easel) {
    let (cx, cy) = (1.0, 1.0);
    let salt = easel.salt;
    let corners = [
        easel.at(0.0, 0.0, 0.0),
        easel.at(2.0, 0.0, 0.0),
        easel.at(2.0, 2.0, 0.0),
        easel.at(0.0, 2.0, 0.0),
    ];
    let anchor = easel.anchor;
    let canvas = &mut easel.canvas;
    paint::fill_polygon(&corners, |px, py| {
        let (fx, fy) = floor_of(anchor, px, py);
        let r = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
        let color = match r {
            r if r > 0.94 => return,
            r if r > 0.84 => rgb(0x3d4a73),
            r if r > 0.76 => rgb(0xe9dcc0),
            r if r > 0.62 => rgb(0xb8623f),
            r if r > 0.54 => rgb(0xe9dcc0),
            r if r > 0.22 => {
                let petal = ((fy - cy).atan2(fx - cx) * 4.0).cos() > 0.55 && r > 0.3;
                if petal { rgb(0xc98a5c) } else { rgb(0xf2e8d2) }
            }
            _ => rgb(0x3d4a73),
        };
        let color = if paint::chance(px, py, salt, 26) {
            paint::darker(color, 0.12)
        } else {
            color
        };
        canvas.set(px, py, color);
    });
}

fn long_rug(easel: &mut Easel) {
    let salt = easel.salt;
    let corners = [
        easel.at(0.0, 0.0, 0.0),
        easel.at(3.0, 0.0, 0.0),
        easel.at(3.0, 2.0, 0.0),
        easel.at(0.0, 2.0, 0.0),
    ];
    let anchor = easel.anchor;
    let canvas = &mut easel.canvas;
    paint::fill_polygon(&corners, |px, py| {
        let (fx, fy) = floor_of(anchor, px, py);
        if !(0.06..2.94).contains(&fx) || !(0.14..1.86).contains(&fy) {
            // A fringe at either end, a few threads standing out.
            let fringe = !(0.06..=2.94).contains(&fx) && (0.18..1.82).contains(&fy);
            if fringe && paint::chance(px, py, salt, 140) {
                canvas.set(px, py, rgb(0xe9dcc0));
            }
            return;
        }
        let band = ((fx - 0.06) / 0.36).floor() as i32;
        let color = match band % 4 {
            0 => rgb(0x8f4a3a),
            1 => rgb(0xe3c27a),
            2 => rgb(0x5f7a8c),
            _ => rgb(0xe9dcc0),
        };
        let edge = !(0.24..=1.76).contains(&fy);
        let color = if edge { rgb(0x4d3a52) } else { color };
        let color = if paint::chance(px, py, salt, 30) {
            paint::darker(color, 0.14)
        } else {
            color
        };
        canvas.set(px, py, color);
    });
}

/// Where on a piece's floor a pixel of its canvas lies.
fn floor_of(anchor: (i32, i32), px: i32, py: i32) -> (f32, f32) {
    let u = (px as f32 + 0.5 - anchor.0 as f32) / 16.0;
    let v = (py as f32 + 0.5 - anchor.1 as f32) / 8.0;
    ((u + v) / 2.0, (v - u) / 2.0)
}

fn lamp(easel: &mut Easel) {
    // A pool of warm light round its foot.
    let (cx, cy) = easel.pixel(0.5, 0.5, 0.0);
    for (rx, alpha) in [(22, 28_u8), (15, 34), (9, 40)] {
        paint::ellipse(&mut easel.canvas, cx, cy, rx, rx / 2, rgba(0xffe2a0, alpha));
    }
    easel.disc((0.5, 0.5), 6.0, 1.0, BRASS.shadow);
    easel.disc((0.48, 0.48), 4.5, 2.0, BRASS.light);
    let (top_x, top_y) = easel.pixel(0.5, 0.5, 34.0);
    paint::vline(&mut easel.canvas, top_x, top_y, cy - top_y - 1, BRASS.base);
    paint::vline(
        &mut easel.canvas,
        top_x - 1,
        top_y,
        cy - top_y - 1,
        BRASS.light,
    );
    easel.cylinder((0.5, 0.5), (9.5, 6.0), (33.0, 45.0), LINEN);
    easel.disc((0.5, 0.5), 6.0, 45.0, LINEN.shine);
    // The bulb's glow under the shade.
    let (gx, gy) = easel.pixel(0.5, 0.5, 33.0);
    paint::ellipse(&mut easel.canvas, gx, gy, 7, 2, rgba(0xfff1c4, 230));
}

fn fern(easel: &mut Easel) {
    easel.shadow((0.2, 0.8), (0.2, 0.8), 55);
    easel.cylinder((0.5, 0.5), (6.0, 7.5), (0.0, 10.0), CLAY);
    easel.disc((0.5, 0.5), 7.5, 10.0, CLAY.light);
    easel.disc((0.5, 0.5), 6.0, 10.0, rgb(0x5a3a28));
    let (cx, cy) = easel.pixel(0.5, 0.5, 10.0);
    // Fronds arching out of the pot, the near ones lighter.
    let fronds: [(f32, f32, f32); 9] = [
        (-1.0, 14.0, 0.9),
        (-0.7, 17.0, 1.0),
        (-0.35, 18.0, 1.0),
        (0.0, 16.0, 0.95),
        (0.35, 18.0, 1.0),
        (0.7, 16.0, 1.0),
        (1.0, 13.0, 0.85),
        (-0.5, 10.0, 0.7),
        (0.55, 10.0, 0.7),
    ];
    for (index, (lean, reach, scale)) in fronds.into_iter().enumerate() {
        let tone = if index >= 7 {
            FERN.light
        } else if index % 2 == 0 {
            FERN.base
        } else {
            FERN.shadow
        };
        let steps = (reach * scale) as i32;
        for step in 0..steps {
            let t = step as f32 / reach;
            let x = cx as f32 + lean * 13.0 * t * scale;
            let y = cy as f32 - reach * scale * (t * 1.6 - t * t * 0.9);
            let (px, py) = (x.round() as i32, y.round() as i32);
            paint::put(&mut easel.canvas, px, py, tone);
            if step % 2 == 0 && step > 1 {
                paint::put(&mut easel.canvas, px - 1, py + 1, FERN.light);
                paint::put(&mut easel.canvas, px + 1, py + 1, tone);
            }
        }
    }
}

fn ball(easel: &mut Easel) {
    easel.shadow((0.3, 0.7), (0.3, 0.7), 60);
    let (cx, cy) = easel.pixel(0.5, 0.5, 6.0);
    let r: i32 = 6;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let lit =
                (-(dx + dy) as f32) / (r as f32 * 1.4) + (1.0 - d2 as f32 / (r * r) as f32) * 0.4;
            let stripe = (dx - dy).abs() <= 1;
            let ramp = if stripe { LINEN } else { BERRY };
            let tone = if lit > 0.55 {
                ramp.shine
            } else if lit > 0.15 {
                ramp.light
            } else if lit > -0.35 {
                ramp.base
            } else {
                ramp.shadow
            };
            easel.canvas.set(cx + dx, cy + dy, tone);
        }
    }
}

fn snack_bowl(easel: &mut Easel) {
    easel.shadow((0.24, 0.76), (0.24, 0.76), 55);
    easel.cylinder((0.5, 0.5), (5.0, 8.0), (0.0, 5.0), GLAZE);
    easel.disc((0.5, 0.5), 8.0, 5.0, GLAZE.light);
    easel.disc((0.5, 0.5), 6.5, 5.0, rgb(0xe8c48a));
    let (cx, cy) = easel.pixel(0.5, 0.5, 5.0);
    for (dx, dy) in [(-3, 0), (-1, -1), (2, 0), (0, 1), (3, -1), (-2, 1)] {
        paint::put(&mut easel.canvas, cx + dx, cy + dy, BERRY.base);
        paint::put(&mut easel.canvas, cx + dx, cy + dy - 1, BERRY.shine);
    }
}

/// A piece this build does not know: a plain crate, so its tile is not mistaken for empty floor.
fn unknown(easel: &mut Easel) {
    easel.shadow((0.15, 0.85), (0.15, 0.85), 50);
    easel.block(Block::new((0.2, 0.8), (0.2, 0.8), (0.0, 10.0), WICKER));
}

/// Every piece at every turn, side by side, for review: `--render-catalog`.
pub fn sheet() -> Canvas {
    use crate::catalog::PIECES;
    let cell = (88, 96);
    let mut sheet = Canvas::new((cell.0 * 4) as u32, (cell.1 * PIECES.len() as i32) as u32);
    for y in 0..sheet.height() as i32 {
        for x in 0..sheet.width() as i32 {
            let row = y / cell.1;
            let tone = if (row + x / cell.0) % 2 == 0 {
                0xeadfcf
            } else {
                0xdfd2bf
            };
            sheet.set(x, y, rgb(tone));
        }
    }
    let mut cache = super::PieceCache::default();
    for (row, piece) in PIECES.iter().enumerate() {
        for turn in 0..4 {
            let sprite = cache.get(piece, turn);
            let (w, d) = piece.size_at(turn);
            let anchor = (
                cell.0 * i32::from(turn) + cell.0 / 2 - (i32::from(w) - i32::from(d)) * 8,
                row as i32 * cell.1 + cell.1 - 14 - (i32::from(w) + i32::from(d)) * 8,
            );
            let (ox, oy) = sprite.origin(anchor);
            paint::blit(&mut sheet, &sprite.canvas, ox, oy);
            if let Some(over) = &sprite.over {
                paint::blit(&mut sheet, over, ox, oy);
            }
        }
    }
    sheet
}
