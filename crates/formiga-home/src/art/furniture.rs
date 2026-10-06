//! Every piece of furniture, drawn on its own footprint from the front (`away` false: its front
//! faces down the room's depth, towards the lower left) or from behind. Each is built from blocks
//! and a few round things, back to front, then edged in a darker shade of its own colours.

use super::ramps::*;
use super::{Block, Easel, Sprite};
use crate::catalog::{Cover, Piece};
use crate::paint::{self, Ramp, rgb, rgba};
use formiga_art::Canvas;

/// A piece's drawing, from the front or from behind.
pub fn draw(piece: &Piece, away: bool) -> Sprite {
    drawn(piece, away).0
}

/// A piece's drawing, and the boards and roofs over its shelves.
fn drawn(piece: &Piece, away: bool) -> (Sprite, Vec<Block>) {
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
        "bed" => bed(&mut easel, flip, away, &BEDDING),
        "driftwood_bed" => bed(&mut easel, flip, away, &SEASIDE_BEDDING),
        "moss_bed" => bed(&mut easel, flip, away, &WOODLAND_BEDDING),
        "night_bed" => bed(&mut easel, flip, away, &STARLIT_BEDDING),
        "cabinet" => over = Some(cabinet(&mut easel, piece)),
        "plinth" => over = Some(plinth(&mut easel, piece)),
        "counter" => over = Some(counter(&mut easel, piece)),
        "deckchair" => over = Some(deckchair(&mut easel, piece, away)),
        "rope_rug" => rope_rug(&mut easel),
        "shell_lamp" => shell_lamp(&mut easel),
        "toadstool" => toadstool(&mut easel),
        "log_table" => log_table(&mut easel),
        "mushroom_lamp" => mushroom_lamp(&mut easel),
        "star_rug" => star_rug(&mut easel),
        "moon_lamp" => moon_lamp(&mut easel),
        "telescope" => telescope(&mut easel, away),
        "basket" => basket(&mut easel),
        "bedroll" => bedroll(&mut easel, flip, away),
        "side_table" => side_table(&mut easel),
        "low_table" => low_table(&mut easel),
        "shelf" => shelf(&mut easel),
        "case" => over = Some(case(&mut easel, piece)),
        "round_rug" => round_rug(&mut easel),
        "long_rug" => long_rug(&mut easel),
        "lamp" => lamp(&mut easel),
        "fern" => fern(&mut easel),
        "ball" => ball(&mut easel),
        "toy_box" => toy_box(&mut easel, away),
        "snack_bowl" => snack_bowl(&mut easel),
        _ => unknown(&mut easel),
    }
    let boards = std::mem::take(&mut easel.boards);
    let (canvas, anchor) = easel.finish();
    let lids = lids(piece, &canvas, &boards);
    let sprite = Sprite {
        canvas,
        anchor,
        over: over.map(|easel| easel.finish().0),
        lids,
    };
    (sprite, boards)
}

/// For each surface of a piece with a board over it, its drawing kept only where that board and
/// any above it are, so that what is shown on the surface can be drawn again behind them.
fn lids(piece: &Piece, canvas: &Canvas, boards: &[Block]) -> Vec<Option<Canvas>> {
    piece
        .surfaces
        .iter()
        .map(|surface| {
            if !matches!(surface.cover, Some(Cover::Board(_))) {
                return None;
            }
            let above: Vec<Block> = boards
                .iter()
                .filter(|board| board.z.0 >= surface.height as f32)
                .copied()
                .collect();
            if above.is_empty() {
                return None;
            }
            let mut mask = Easel::new(piece.size.0, piece.size.1, piece.height.max(8) + 4, 0);
            for board in above {
                mask.block(board);
            }
            let mut lid = Canvas::new(canvas.width(), canvas.height());
            for y in 0..canvas.height() as i32 {
                for x in 0..canvas.width() as i32 {
                    if mask.canvas.get(x, y).a > 0 {
                        lid.set(x, y, canvas.get(x, y));
                    }
                }
            }
            Some(lid)
        })
        .collect()
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

/// What a bed is made of, and what its blanket shows on top.
struct Bedding {
    head: Ramp,
    frame: Ramp,
    sheet: Ramp,
    blanket: Ramp,
    pattern: Pattern,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pattern {
    /// A plain blanket with its hem turned down.
    Hem,
    /// Stripes across, like a beach towel.
    Stripes,
    /// Little leaves scattered over it.
    Leaves,
    /// Gold stars on the night sky.
    Stars,
}

const BEDDING: Bedding = Bedding {
    head: WALNUT,
    frame: OAK,
    sheet: LINEN,
    blanket: ROSE,
    pattern: Pattern::Hem,
};

const SEASIDE_BEDDING: Bedding = Bedding {
    head: DRIFTWOOD,
    frame: DRIFTWOOD,
    sheet: LINEN,
    blanket: SEA,
    pattern: Pattern::Stripes,
};

const WOODLAND_BEDDING: Bedding = Bedding {
    head: BARK,
    frame: BARK,
    sheet: MOSS,
    blanket: SAGE,
    pattern: Pattern::Leaves,
};

const STARLIT_BEDDING: Bedding = Bedding {
    head: WALNUT,
    frame: WALNUT,
    sheet: LINEN,
    blanket: NIGHT,
    pattern: Pattern::Stars,
};

fn bed(easel: &mut Easel, flip: impl Fn(Block) -> Block, away: bool, bedding: &Bedding) {
    easel.shadow((0.06, 0.94), (0.04, 1.96), 70);
    let mut blocks = feet((0.1, 0.9), (0.1, 1.9), 3.0, 0.0);
    blocks.extend([
        // The headboard at the head, the far end, with the frame, the mattress and the
        // blanket in front of it.
        Block::new((0.06, 0.94), (0.04, 0.16), (2.0, 19.0), bedding.head),
        Block::new((0.1, 0.9), (0.16, 1.9), (3.0, 7.0), bedding.frame),
        Block::new((0.13, 0.87), (0.18, 1.86), (7.0, 10.0), bedding.sheet),
        Block::new((0.13, 0.87), (0.72, 1.88), (10.0, 12.0), bedding.blanket),
        Block::new((0.08, 0.92), (1.84, 1.95), (2.0, 10.0), bedding.head),
    ]);
    easel.blocks(blocks.into_iter().map(&flip).collect());
    // The pillow, plump at the head; and the blanket's turned-down hem.
    let head = if away { 1.62 } else { 0.38 };
    easel.disc((0.5, head), 8.0, 11.5, LINEN.light);
    easel.disc((0.46, head - 0.03), 5.0, 12.5, LINEN.shine);
    let hem = if away { 1.18 } else { 0.76 };
    let (a, b) = (easel.pixel(0.14, hem, 12.0), easel.pixel(0.86, hem, 12.0));
    paint::line(&mut easel.canvas, a, b, bedding.blanket.shine);
    if bedding.pattern == Pattern::Hem {
        return;
    }
    // The blanket's pattern, over its top.
    let (y0, y1) = if away { (0.12, 1.28) } else { (0.72, 1.88) };
    let top = [
        easel.at(0.13, y0, 12.0),
        easel.at(0.87, y0, 12.0),
        easel.at(0.87, y1, 12.0),
        easel.at(0.13, y1, 12.0),
    ];
    let (anchor, salt) = (easel.anchor, easel.salt);
    let ramp = bedding.blanket;
    let canvas = &mut easel.canvas;
    paint::fill_polygon(&top, |px, py| {
        let (fx, fy) = floor_of(anchor, px, py + 12);
        let mark = match bedding.pattern {
            Pattern::Stripes => ((fy * 7.0).floor() as i32 % 2 == 0).then_some(LINEN.light),
            Pattern::Leaves => (paint::chance(px, py, salt, 30) && (px + py) % 2 == 0)
                .then_some(if fx < 0.5 { MOSS.light } else { MOSS.base }),
            Pattern::Stars => paint::chance(px, py, salt, 9).then_some(BRASS.shine),
            Pattern::Hem => None,
        };
        if let Some(mark) = mark
            && (0.18..0.82).contains(&fx)
        {
            canvas.set(px, py, mark);
        } else if bedding.pattern == Pattern::Stars && paint::chance(px, py, salt + 1, 6) {
            canvas.set(px, py, ramp.light);
        }
    });
}

/// A guest's bedroll: a quilted mat rolled out on the floor, a pillow at its head and the end of
/// it still rolled up at its foot.
fn bedroll(easel: &mut Easel, flip: impl Fn(Block) -> Block, away: bool) {
    easel.shadow((0.1, 0.9), (0.08, 1.92), 50);
    let blocks = vec![
        Block::new((0.12, 0.88), (0.12, 1.66), (0.0, 2.0), CORNFLOWER),
        Block::new((0.1, 0.9), (1.62, 1.92), (0.0, 6.0), CORNFLOWER),
    ];
    easel.blocks(blocks.into_iter().map(&flip).collect());
    // The quilting: a stitched line across the mat every so often.
    for along in [0.62, 0.92, 1.22] {
        let y = if away { 1.92 - along + 0.04 } else { along };
        let (a, b) = (easel.pixel(0.16, y, 2.0), easel.pixel(0.84, y, 2.0));
        paint::line(&mut easel.canvas, a, b, CORNFLOWER.base);
    }
    // The rolled end's spiral, on its face towards the mat.
    let roll = if away { 0.38 } else { 1.62 };
    let (cx, cy) = easel.pixel(0.5, roll, 3.0);
    paint::put(&mut easel.canvas, cx, cy, CORNFLOWER.shine);
    paint::put(&mut easel.canvas, cx + 1, cy, CORNFLOWER.shadow);
    let head = if away { 1.58 } else { 0.4 };
    easel.disc((0.5, head), 7.0, 3.0, LINEN.light);
    easel.disc((0.46, head - 0.03), 4.0, 4.0, LINEN.shine);
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

/// An open shelf: posts at its three far corners and three boards between them, open at the near
/// corner so nothing on it is ever cut in two, and far enough apart that what stands on one
/// clears the next.
fn shelf(easel: &mut Easel) {
    easel.shadow((0.1, 0.9), (0.1, 0.9), 60);
    let post = |x: f32, y: f32| Block::new((x, x + 0.11), (y, y + 0.11), (0.0, 44.0), OAK);
    let board = |z: f32| Block::new((0.12, 0.88), (0.12, 0.88), (z - 2.0, z), OAK);
    let mut blocks = vec![post(0.12, 0.12), post(0.77, 0.12), post(0.12, 0.77)];
    let boards = [board(4.0), board(24.0), board(44.0)];
    blocks.extend(boards);
    easel.blocks(blocks);
    easel.boards.extend(boards);
}

/// A glass case on a plinth, lined in velvet like Formiga Hill's own. Its glass is drawn in front
/// of what is shown in it.
fn case(easel: &mut Easel, piece: &Piece) -> Easel {
    easel.shadow((0.08, 0.92), (0.08, 0.92), 60);
    easel.block(Block::new((0.1, 0.9), (0.1, 0.9), (0.0, 5.0), WALNUT));
    // The velvet lining the two far sides, seen through the glass.
    let back_x = [
        easel.at(0.12, 0.12, 41.0),
        easel.at(0.12, 0.88, 41.0),
        easel.at(0.12, 0.88, 6.0),
        easel.at(0.12, 0.12, 6.0),
    ];
    let back_y = [
        easel.at(0.12, 0.12, 41.0),
        easel.at(0.88, 0.12, 41.0),
        easel.at(0.88, 0.12, 6.0),
        easel.at(0.12, 0.12, 6.0),
    ];
    paint::polygon(&mut easel.canvas, &back_x, VELVET.shadow);
    paint::polygon(&mut easel.canvas, &back_y, VELVET.base);
    let boards = [
        Block::new((0.13, 0.87), (0.13, 0.87), (5.0, 6.0), LINEN),
        Block::new((0.13, 0.87), (0.13, 0.87), (23.0, 24.0), LINEN),
        Block::new((0.1, 0.9), (0.1, 0.9), (41.0, 44.0), WALNUT),
    ];
    for board in boards {
        easel.block(board);
    }
    easel.boards.extend(boards);
    let mut glass = Easel::new(
        piece.size.0,
        piece.size.1,
        piece.height.max(8) + 4,
        easel.salt,
    );
    let pane = rgba(0xd8f0f2, 70);
    let front_left = [
        glass.at(0.1, 0.9, 41.0),
        glass.at(0.9, 0.9, 41.0),
        glass.at(0.9, 0.9, 6.0),
        glass.at(0.1, 0.9, 6.0),
    ];
    let front_right = [
        glass.at(0.9, 0.1, 41.0),
        glass.at(0.9, 0.9, 41.0),
        glass.at(0.9, 0.9, 6.0),
        glass.at(0.9, 0.1, 6.0),
    ];
    paint::polygon(&mut glass.canvas, &front_left, pane);
    paint::polygon(&mut glass.canvas, &front_right, rgba(0xc4e2e6, 80));
    // The panes' meeting edge, a bright line rather than a post, so it never hides what is
    // shown; and a glint across each pane.
    let (top, bottom) = (glass.pixel(0.9, 0.9, 40.0), glass.pixel(0.9, 0.9, 7.0));
    paint::line(&mut glass.canvas, top, bottom, rgba(0xf4fcfc, 120));
    for (from, to) in [
        (glass.pixel(0.25, 0.9, 36.0), glass.pixel(0.45, 0.9, 15.0)),
        (glass.pixel(0.9, 0.3, 37.0), glass.pixel(0.9, 0.42, 26.0)),
    ] {
        paint::line(&mut glass.canvas, from, to, rgba(0xffffff, 150));
    }
    glass
}

/// A tall glass-fronted cabinet for the colony's best things: walnut, crowned along its top, its
/// three shelves lined in velvet. Like the case, its panes meet at the near corner in a bright
/// line rather than a post, and go in front of whatever is shown in it.
fn cabinet(easel: &mut Easel, piece: &Piece) -> Easel {
    easel.shadow((0.08, 0.92), (0.08, 0.92), 65);
    easel.block(Block::new((0.1, 0.9), (0.1, 0.9), (0.0, 3.0), WALNUT));
    let back_x = [
        easel.at(0.12, 0.12, 51.0),
        easel.at(0.12, 0.88, 51.0),
        easel.at(0.12, 0.88, 3.0),
        easel.at(0.12, 0.12, 3.0),
    ];
    let back_y = [
        easel.at(0.12, 0.12, 51.0),
        easel.at(0.88, 0.12, 51.0),
        easel.at(0.88, 0.12, 3.0),
        easel.at(0.12, 0.12, 3.0),
    ];
    paint::polygon(&mut easel.canvas, &back_x, VELVET.shadow);
    paint::polygon(&mut easel.canvas, &back_y, VELVET.base);
    for (x, y) in [(0.1, 0.1), (0.8, 0.1), (0.1, 0.8)] {
        easel.block(Block::new((x, x + 0.1), (y, y + 0.1), (3.0, 51.0), WALNUT));
    }
    let boards = [
        Block::new((0.13, 0.87), (0.13, 0.87), (3.0, 4.0), LINEN),
        Block::new((0.13, 0.87), (0.13, 0.87), (19.0, 20.0), LINEN),
        Block::new((0.13, 0.87), (0.13, 0.87), (35.0, 36.0), LINEN),
        Block::new((0.08, 0.92), (0.08, 0.92), (51.0, 53.0), WALNUT),
        Block::new((0.05, 0.95), (0.05, 0.95), (53.0, 55.0), WALNUT),
    ];
    for board in boards {
        easel.block(board);
    }
    easel.boards.extend(boards);
    let (fx, fy) = easel.pixel(0.5, 0.5, 56.0);
    paint::put(&mut easel.canvas, fx, fy, BRASS.light);
    paint::put(&mut easel.canvas, fx, fy - 1, BRASS.shine);
    glass_front(piece, easel.salt, (0.1, 0.9), (0.1, 0.9), (3.0, 51.0))
}

/// The panes of a glass-fronted piece: its two near faces, meeting in a bright line, with a glint
/// across each.
fn glass_front(piece: &Piece, salt: u32, x: (f32, f32), y: (f32, f32), z: (f32, f32)) -> Easel {
    let mut glass = Easel::new(piece.size.0, piece.size.1, piece.height.max(8) + 4, salt);
    let front_left = [
        glass.at(x.0, y.1, z.1),
        glass.at(x.1, y.1, z.1),
        glass.at(x.1, y.1, z.0),
        glass.at(x.0, y.1, z.0),
    ];
    let front_right = [
        glass.at(x.1, y.0, z.1),
        glass.at(x.1, y.1, z.1),
        glass.at(x.1, y.1, z.0),
        glass.at(x.1, y.0, z.0),
    ];
    paint::polygon(&mut glass.canvas, &front_left, rgba(0xd8f0f2, 70));
    paint::polygon(&mut glass.canvas, &front_right, rgba(0xc4e2e6, 80));
    let (top, bottom) = (
        glass.pixel(x.1, y.1, z.1 - 1.0),
        glass.pixel(x.1, y.1, z.0 + 1.0),
    );
    paint::line(&mut glass.canvas, top, bottom, rgba(0xf4fcfc, 120));
    let tall = z.1 - z.0;
    for (from, to) in [
        (
            glass.pixel(x.0 + (x.1 - x.0) * 0.2, y.1, z.1 - tall * 0.15),
            glass.pixel(x.0 + (x.1 - x.0) * 0.42, y.1, z.0 + tall * 0.25),
        ),
        (
            glass.pixel(x.1, y.0 + (y.1 - y.0) * 0.25, z.1 - tall * 0.12),
            glass.pixel(x.1, y.0 + (y.1 - y.0) * 0.4, z.1 - tall * 0.45),
        ),
    ] {
        paint::line(&mut glass.canvas, from, to, rgba(0xffffff, 150));
    }
    glass
}

/// A white plinth with a bell jar on it, for one thing on its own: the jar goes over what it
/// keeps.
fn plinth(easel: &mut Easel, piece: &Piece) -> Easel {
    easel.shadow((0.18, 0.82), (0.18, 0.82), 55);
    easel.blocks(vec![
        Block::new((0.22, 0.78), (0.22, 0.78), (0.0, 3.0), WALNUT),
        Block::new((0.3, 0.7), (0.3, 0.7), (3.0, 17.0), LINEN),
        Block::new((0.24, 0.76), (0.24, 0.76), (17.0, 20.0), WALNUT),
    ]);
    let mut glass = Easel::new(
        piece.size.0,
        piece.size.1,
        piece.height.max(8) + 4,
        easel.salt,
    );
    let (cx, base) = glass.pixel(0.5, 0.5, 20.0);
    let (r, side) = (7, 10);
    for up in 0..=side + r {
        // Straight up the sides, then round over the top.
        let half = if up <= side {
            r
        } else {
            let over = (up - side) as f32 / r as f32;
            ((1.0 - over * over).max(0.0).sqrt() * r as f32).round() as i32
        };
        let y = base - up;
        for dx in -half..=half {
            let edge = dx.abs() == half;
            let alpha = if edge { 130 } else { 50 };
            paint::put(&mut glass.canvas, cx + dx, y, rgba(0xd8f0f2, alpha));
        }
    }
    // A glint down its lit side, and a knob on top.
    paint::vline(
        &mut glass.canvas,
        cx - 4,
        base - side - 2,
        side - 2,
        rgba(0xffffff, 170),
    );
    paint::put(&mut glass.canvas, cx, base - side - r - 1, BRASS.light);
    paint::put(&mut glass.canvas, cx, base - side - r - 2, BRASS.shine);
    glass
}

/// A long, low museum counter of oak, its top a glass case lined in velvet for small things.
fn counter(easel: &mut Easel, piece: &Piece) -> Easel {
    easel.shadow((0.04, 1.96), (0.06, 0.94), 60);
    easel.block(Block::new((0.06, 1.94), (0.1, 0.9), (0.0, 9.0), OAK));
    let floor = [
        easel.at(0.1, 0.14, 9.0),
        easel.at(1.9, 0.14, 9.0),
        easel.at(1.9, 0.86, 9.0),
        easel.at(0.1, 0.86, 9.0),
    ];
    paint::polygon(&mut easel.canvas, &floor, VELVET.base);
    // The far panes, faint, and the oak frame along the top at the back.
    let back = [
        easel.at(0.08, 0.12, 25.0),
        easel.at(1.92, 0.12, 25.0),
        easel.at(1.92, 0.12, 9.0),
        easel.at(0.08, 0.12, 9.0),
    ];
    paint::polygon(&mut easel.canvas, &back, rgba(0xc4e2e6, 40));
    let (a, b) = (easel.pixel(0.08, 0.12, 25.0), easel.pixel(1.92, 0.12, 25.0));
    paint::line(&mut easel.canvas, a, b, OAK.base);
    let mut glass = glass_front(piece, easel.salt, (0.08, 1.92), (0.12, 0.88), (9.0, 25.0));
    // The glass top, and a brass edge round it.
    let top = [
        glass.at(0.08, 0.12, 25.0),
        glass.at(1.92, 0.12, 25.0),
        glass.at(1.92, 0.88, 25.0),
        glass.at(0.08, 0.88, 25.0),
    ];
    paint::polygon(&mut glass.canvas, &top, rgba(0xe8f6f8, 70));
    // A brass frame round the glass: along its top edges and up its corners.
    for (from, to) in [
        ((0.08, 0.88, 25.0), (1.92, 0.88, 25.0)),
        ((1.92, 0.88, 25.0), (1.92, 0.12, 25.0)),
        ((0.08, 0.88, 25.0), (0.08, 0.88, 9.0)),
        ((1.92, 0.12, 25.0), (1.92, 0.12, 9.0)),
    ] {
        let (a, b) = (
            glass.pixel(from.0, from.1, from.2),
            glass.pixel(to.0, to.1, to.2),
        );
        paint::line(&mut glass.canvas, a, b, BRASS.light);
    }
    glass
}

/// A deckchair: a driftwood frame with a striped sling slung in it, low at the back and leaning
/// back. The near rail is in front of whoever sits in it; turned away, so is its back.
fn deckchair(easel: &mut Easel, piece: &Piece, away: bool) -> Easel {
    easel.shadow((0.12, 0.88), (0.08, 0.92), 55);
    let flip = |block: Block| if away { block.facing_away(1.0) } else { block };
    let mut behind = vec![
        flip(Block::new((0.12, 0.2), (0.12, 0.92), (0.0, 6.0), DRIFTWOOD)),
        flip(Block::new(
            (0.12, 0.2),
            (0.06, 0.16),
            (0.0, 21.0),
            DRIFTWOOD,
        )),
    ];
    let mut front = vec![
        flip(Block::new((0.8, 0.88), (0.12, 0.92), (0.0, 6.0), DRIFTWOOD)),
        flip(Block::new(
            (0.8, 0.88),
            (0.06, 0.16),
            (0.0, 21.0),
            DRIFTWOOD,
        )),
    ];
    let mut back = Vec::new();
    for slice in 0..6 {
        let x0 = 0.2 + slice as f32 * 0.1;
        let cloth = if slice % 2 == 0 { SEA } else { LINEN };
        behind.push(flip(Block::new(
            (x0, x0 + 0.1),
            (0.34, 0.86),
            (4.0, 6.0),
            cloth,
        )));
        back.push(flip(Block::new(
            (x0, x0 + 0.1),
            (0.22, 0.34),
            (6.0, 12.0),
            cloth,
        )));
        back.push(flip(Block::new(
            (x0, x0 + 0.1),
            (0.12, 0.22),
            (12.0, 19.0),
            cloth,
        )));
    }
    if away {
        front.extend(back);
    } else {
        behind.extend(back);
    }
    easel.blocks(behind);
    let mut over = Easel::new(
        piece.size.0,
        piece.size.1,
        piece.height.max(8) + 4,
        easel.salt,
    );
    over.blocks(front);
    over
}

/// A rug of rope coiled round and round, sewn flat.
fn rope_rug(easel: &mut Easel) {
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
        let r = ((fx - 1.0).powi(2) + (fy - 1.0).powi(2)).sqrt();
        if r > 0.92 {
            return;
        }
        let coil = (r * 11.0).fract();
        let color = if coil < 0.2 {
            WICKER.shadow
        } else if coil < 0.5 {
            WICKER.light
        } else if r > 0.82 {
            SEA.base
        } else {
            WICKER.base
        };
        let color = if paint::chance(px, py, salt, 30) {
            paint::darker(color, 0.12)
        } else {
            color
        };
        canvas.set(px, py, color);
    });
}

/// A table lamp with a shade like a scallop shell, on a driftwood stem.
fn shell_lamp(easel: &mut Easel) {
    easel.shadow((0.3, 0.7), (0.3, 0.7), 50);
    easel.disc((0.5, 0.5), 5.0, 1.0, DRIFTWOOD.shadow);
    easel.disc((0.48, 0.48), 3.5, 2.0, DRIFTWOOD.light);
    let (cx, foot) = easel.pixel(0.5, 0.5, 2.0);
    let (_, hinge) = easel.pixel(0.5, 0.5, 16.0);
    paint::vline(&mut easel.canvas, cx, hinge, foot - hinge, DRIFTWOOD.base);
    paint::vline(
        &mut easel.canvas,
        cx - 1,
        hinge,
        foot - hinge,
        DRIFTWOOD.light,
    );
    // The shell: ribs fanning up and out from its hinge, scalloped at its rim.
    let radius = 12.0;
    for dy in -13_i32..=0 {
        for dx in -13_i32..=13 {
            let (fx, fy) = (dx as f32, -dy as f32 * 1.15);
            let r = (fx * fx + fy * fy).sqrt();
            let angle = fy.atan2(fx);
            let rim = radius - 1.2 * (1.0 - (angle * 9.0).sin().abs());
            if r > rim || fy < 0.5 {
                continue;
            }
            let rib = ((angle / std::f32::consts::PI) * 9.0).floor() as i32;
            let tone = if ((angle / std::f32::consts::PI) * 9.0).fract() < 0.18 {
                SHELL.shadow
            } else if rib % 2 == 0 {
                SHELL.light
            } else {
                SHELL.base
            };
            let tone = if r > rim - 1.0 { SHELL.edge } else { tone };
            easel.canvas.set(cx + dx, hinge + dy, tone);
        }
    }
}

/// A toadstool to sit on: a red cap with white spots on a stout cream stem.
fn toadstool(easel: &mut Easel) {
    easel.shadow((0.16, 0.84), (0.16, 0.84), 55);
    easel.cylinder((0.5, 0.5), (4.5, 3.5), (0.0, 8.0), LINEN);
    easel.cylinder((0.5, 0.5), (11.0, 10.5), (8.0, 11.0), BERRY);
    easel.disc((0.5, 0.5), 10.5, 11.0, BERRY.light);
    easel.disc((0.47, 0.46), 6.0, 12.5, BERRY.shine);
    let (cx, cy) = easel.pixel(0.5, 0.5, 12.0);
    for (dx, dy) in [(-6, 0), (-2, -2), (3, -1), (6, 1), (0, 2), (-4, 2)] {
        paint::put(&mut easel.canvas, cx + dx, cy + dy, LINEN.shine);
        paint::put(&mut easel.canvas, cx + dx + 1, cy + dy, LINEN.light);
    }
}

/// A sawn log for a table: bark round its sides and its rings on top.
fn log_table(easel: &mut Easel) {
    easel.shadow((0.16, 0.84), (0.16, 0.84), 55);
    easel.cylinder((0.5, 0.5), (10.0, 10.0), (0.0, 12.0), BARK);
    // The bark's furrows.
    let (cx, foot) = easel.pixel(0.5, 0.5, 0.0);
    for dx in [-7, -3, 2, 6] {
        for up in 1..11 {
            if paint::chance(cx + dx, foot - up, easel.salt, 170) {
                let y = foot - up + ((dx.abs() as f32 / 3.0) as i32);
                paint::put(&mut easel.canvas, cx + dx, y, BARK.edge);
            }
        }
    }
    for (r, tone) in [
        (10.0, WICKER.light),
        (8.0, WICKER.base),
        (6.5, WICKER.light),
        (4.5, WICKER.base),
        (2.5, WICKER.light),
        (1.0, WICKER.shadow),
    ] {
        easel.disc((0.5, 0.5), r, 12.0, tone);
    }
}

/// A lamp shaped like a mushroom, its cap the shade, with a little one beside it.
fn mushroom_lamp(easel: &mut Easel) {
    easel.shadow((0.18, 0.82), (0.18, 0.82), 50);
    easel.cylinder((0.24, 0.72), (2.0, 1.5), (0.0, 5.0), LINEN);
    easel.cylinder((0.24, 0.72), (4.0, 3.0), (5.0, 8.0), PEACH);
    easel.disc((0.24, 0.72), 3.0, 8.0, PEACH.light);
    easel.cylinder((0.5, 0.5), (3.5, 2.5), (0.0, 13.0), LINEN);
    easel.cylinder((0.5, 0.5), (9.0, 6.0), (13.0, 21.0), PEACH);
    easel.disc((0.5, 0.5), 6.0, 21.0, PEACH.light);
    easel.disc((0.47, 0.47), 3.0, 22.0, PEACH.shine);
    let (cx, cy) = easel.pixel(0.5, 0.5, 17.0);
    for (dx, dy) in [(-5, 1), (-1, -1), (4, 0), (2, 2)] {
        paint::put(&mut easel.canvas, cx + dx, cy + dy, LINEN.shine);
    }
}

/// A rug the colour of the night, edged in gold, with stars on it.
fn star_rug(easel: &mut Easel) {
    let salt = easel.salt;
    let corners = [
        easel.at(0.0, 0.0, 0.0),
        easel.at(2.0, 0.0, 0.0),
        easel.at(2.0, 2.0, 0.0),
        easel.at(0.0, 2.0, 0.0),
    ];
    let anchor = easel.anchor;
    let stars = [
        (0.55, 0.6),
        (1.35, 0.5),
        (1.0, 1.05),
        (0.5, 1.45),
        (1.5, 1.42),
    ];
    let canvas = &mut easel.canvas;
    paint::fill_polygon(&corners, |px, py| {
        let (fx, fy) = floor_of(anchor, px, py);
        let inset = fx.min(fy).min(2.0 - fx).min(2.0 - fy);
        if inset < 0.08 {
            return;
        }
        let star = stars.iter().any(|&(sx, sy)| {
            let (dx, dy) = ((fx - sx).abs(), (fy - sy).abs());
            (dx < 0.05 && dy < 0.17) || (dy < 0.05 && dx < 0.17) || (dx + dy < 0.1)
        });
        let color = if inset < 0.15 {
            BRASS.light
        } else if star {
            BRASS.shine
        } else if paint::chance(px, py, salt, 6) {
            MOON.light
        } else if paint::chance(px, py, salt + 1, 40) {
            NIGHT.shadow
        } else {
            NIGHT.base
        };
        canvas.set(px, py, color);
    });
}

/// A lamp that is a little moon on a brass stand, craters and all.
fn moon_lamp(easel: &mut Easel) {
    easel.shadow((0.28, 0.72), (0.28, 0.72), 50);
    easel.disc((0.5, 0.5), 5.0, 1.0, BRASS.shadow);
    easel.disc((0.48, 0.48), 3.5, 2.0, BRASS.light);
    let (cx, foot) = easel.pixel(0.5, 0.5, 2.0);
    let (_, top) = easel.pixel(0.5, 0.5, 21.0);
    paint::vline(&mut easel.canvas, cx, top, foot - top, BRASS.base);
    let (mx, my) = easel.pixel(0.5, 0.5, 28.0);
    let r: i32 = 8;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let lit =
                (-(dx + dy) as f32) / (r as f32 * 1.4) + (1.0 - d2 as f32 / (r * r) as f32) * 0.4;
            let tone = if lit > 0.55 {
                MOON.shine
            } else if lit > 0.15 {
                MOON.light
            } else if lit > -0.35 {
                MOON.base
            } else {
                MOON.shadow
            };
            easel.canvas.set(mx + dx, my + dy, tone);
        }
    }
    for (dx, dy) in [(-3, -2), (2, 1), (-1, 3), (4, -3)] {
        paint::put(&mut easel.canvas, mx + dx, my + dy, MOON.shadow);
        paint::put(&mut easel.canvas, mx + dx + 1, my + dy + 1, MOON.light);
    }
}

/// A toy telescope on a walnut tripod, pointed up at the sky beyond the far wall.
fn telescope(easel: &mut Easel, away: bool) {
    easel.shadow((0.18, 0.82), (0.18, 0.82), 50);
    let mount = easel.pixel(0.5, 0.5, 16.0);
    let legs = if away {
        [(0.22, 0.24), (0.78, 0.24), (0.5, 0.82)]
    } else {
        [(0.22, 0.76), (0.78, 0.76), (0.5, 0.18)]
    };
    for (x, y) in legs {
        let foot = easel.pixel(x, y, 0.0);
        paint::line(&mut easel.canvas, mount, foot, WALNUT.base);
        paint::line(
            &mut easel.canvas,
            (mount.0 - 1, mount.1),
            (foot.0 - 1, foot.1),
            WALNUT.light,
        );
    }
    // The tube, from the eyepiece over the mount to the lens, wide enough to keep its brass
    // inside its edge, and wider at the lens.
    let far = if away { 0.92 } else { 0.08 };
    let (from, to) = (easel.at(0.5, 0.64, 15.0), easel.at(0.5, far, 30.0));
    let steps = 40;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let (x, y) = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        let (px, py) = (x.round() as i32, y.round() as i32);
        let wide = if t > 0.82 { 3 } else { 2 };
        for dx in -1..=1 {
            for dy in -wide..=wide {
                let tone = match dy {
                    d if d < -1 => BRASS.shine,
                    d if d < 0 => BRASS.light,
                    d if d < wide => BRASS.base,
                    _ => BRASS.shadow,
                };
                paint::put(&mut easel.canvas, px + dx, py + dy, tone);
            }
        }
    }
    // A band round the tube where it rests on the mount.
    let (bx, by) = (
        (from.0 + (to.0 - from.0) * 0.3).round() as i32,
        (from.1 + (to.1 - from.1) * 0.3).round() as i32,
    );
    paint::vline(&mut easel.canvas, bx, by - 2, 5, WALNUT.base);
    let lens = (to.0.round() as i32, to.1.round() as i32);
    paint::put(&mut easel.canvas, lens.0, lens.1, GLAZE.light);
    paint::put(&mut easel.canvas, lens.0, lens.1 - 1, GLAZE.shine);
    let eye = (from.0.round() as i32, from.1.round() as i32);
    paint::put(&mut easel.canvas, eye.0, eye.1, WALNUT.edge);
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

/// A floor lamp, unlit: the scene adds its light while it is on.
fn lamp(easel: &mut Easel) {
    let (_, cy) = easel.pixel(0.5, 0.5, 0.0);
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
}

/// A lamp's light, centred on the foot of the lamp at `foot` on the scene: a warm pool round
/// its foot, drawn on the floor before anything stands on it.
pub fn lamp_pool(canvas: &mut Canvas, foot: (i32, i32)) {
    for (rx, alpha) in [(22, 28_u8), (15, 34), (9, 40)] {
        paint::ellipse(canvas, foot.0, foot.1, rx, rx / 2, rgba(0xffe2a0, alpha));
    }
}

/// The glow of the bulb under a lit lamp's shade, `glow` pixels above its foot.
pub fn lamp_bulb(canvas: &mut Canvas, foot: (i32, i32), glow: i32) {
    paint::ellipse(canvas, foot.0, foot.1 - glow, 7, 2, rgba(0xfff1c4, 230));
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

/// A painted chest with its lid propped open at the back and toys peeking over the rim. From
/// behind, the open lid stands in front of them.
fn toy_box(easel: &mut Easel, away: bool) {
    easel.shadow((0.12, 0.88), (0.14, 0.88), 60);
    let flip = |block: Block| if away { block.facing_away(1.0) } else { block };
    let body = Block::new((0.14, 0.86), (0.2, 0.86), (0.0, 12.0), BERRY);
    let lid = Block::new((0.12, 0.88), (0.1, 0.2), (11.0, 22.0), BERRY);
    if away {
        easel.block(flip(body));
    } else {
        easel.blocks(vec![lid, body]);
    }
    // A thin brass band round it.
    let corner = easel.pixel(0.86, 0.86, 6.0);
    let (left, right) = (easel.pixel(0.14, 0.86, 6.0), easel.pixel(0.86, 0.2, 6.0));
    paint::line(&mut easel.canvas, left, corner, BRASS.light);
    paint::line(&mut easel.canvas, corner, right, BRASS.base);
    // The dark inside, and the toys in it.
    let inside = [
        easel.at(0.2, 0.26, 12.0),
        easel.at(0.8, 0.26, 12.0),
        easel.at(0.8, 0.8, 12.0),
        easel.at(0.2, 0.8, 12.0),
    ];
    paint::polygon(&mut easel.canvas, &inside, BERRY.edge);
    let (bx, by) = easel.pixel(0.4, 0.5, 14.0);
    paint::ellipse(&mut easel.canvas, bx, by, 3, 3, GLAZE.light);
    paint::put(&mut easel.canvas, bx - 1, by - 1, GLAZE.shine);
    let (sx, sy) = easel.pixel(0.64, 0.42, 12.0);
    paint::vline(&mut easel.canvas, sx, sy - 9, 9, WALNUT.light);
    for (dx, dy, color) in [
        (-2, -11, SAGE.light),
        (2, -11, ROSE.light),
        (0, -13, BRASS.light),
    ] {
        paint::put(&mut easel.canvas, sx + dx, sy + dy, color);
        paint::put(&mut easel.canvas, sx + dx / 2, sy + dy + 1, color);
    }
    if away {
        easel.block(flip(lid));
    } else {
        // A painted star on its front.
        let (cx, cy) = easel.pixel(0.5, 0.86, 9.0);
        for (dx, dy) in [(0, -1), (-1, 0), (0, 0), (1, 0), (0, 1)] {
            paint::put(&mut easel.canvas, cx + dx, cy + dy, BRASS.shine);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PIECES;

    #[test]
    fn every_shelf_under_a_board_has_the_room_the_catalogue_gives_it_and_is_drawn_behind_it() {
        for piece in &PIECES {
            let (sprite, boards) = drawn(piece, false);
            for (slot, surface) in piece.surfaces.iter().enumerate() {
                let Some(Cover::Board(room)) = surface.cover else {
                    assert!(sprite.lids[slot].is_none(), "{} {slot}", piece.id);
                    continue;
                };
                let lowest_above = boards
                    .iter()
                    .map(|board| board.z.0)
                    .filter(|&bottom| bottom >= surface.height as f32)
                    .min_by(f32::total_cmp);
                assert_eq!(
                    lowest_above,
                    Some((surface.height + room) as f32),
                    "{} {slot}",
                    piece.id
                );
                assert!(sprite.lids[slot].is_some(), "{} {slot}", piece.id);
            }
        }
    }

    #[test]
    fn every_piece_has_a_drawing_of_its_own_from_the_front_and_from_behind() {
        let crate_of = |piece: &Piece| {
            let mut easel = Easel::new(piece.size.0, piece.size.1, piece.height.max(8) + 4, 1);
            unknown(&mut easel);
            easel.finish().0
        };
        for piece in &PIECES {
            for away in [false, true] {
                let sprite = draw(piece, away);
                assert!(
                    sprite.canvas.alpha_bounds().is_some(),
                    "{} draws nothing",
                    piece.id
                );
                assert_ne!(
                    sprite.canvas,
                    crate_of(piece),
                    "{} is drawn as a piece this build does not know",
                    piece.id
                );
            }
        }
    }

    #[test]
    fn whatever_is_shown_on_a_piece_is_shown_within_its_picture() {
        for piece in &PIECES {
            let sprite = draw(piece, false);
            let (l, t, r, b) = sprite.canvas.alpha_bounds().unwrap();
            let easel = Easel::new(piece.size.0, piece.size.1, piece.height.max(8) + 4, 1);
            for surface in piece.surfaces {
                let (x, y) = easel.pixel(surface.at.0, surface.at.1, surface.height as f32);
                assert!(
                    (l as i32..=r as i32).contains(&x) && (t as i32..=b as i32).contains(&y),
                    "{}: a surface at {:?} is off its picture",
                    piece.id,
                    surface.at
                );
            }
        }
    }
}
