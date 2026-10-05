//! The notebook Home is kept in. Its leather cover, stitched round the edge with brass at the
//! corners, is the window's own frame; open on it are two pages, the house drawn on the left one
//! and notes kept on the right. A cloth patch on the cover carries the house's name, the two modes
//! are tabs stitched to the cover, and the window's own buttons are brass studs.
//!
//! The notebook is painted to the rooms' rules, at the same pixel scale: every material a ramp,
//! light from the upper left, edges in a darker shade of their own colour, never black, and grain
//! from `paint::noise`. Only lettering is left to the window, written over what is painted here.

use crate::paint::{self, Ramp, rgb};
use formiga_art::{Canvas, Rgba};

/// A rectangle of the notebook, in its own pixels: left, top, width, height.
pub type Area = (i32, i32, i32, i32);

/// The notebook's materials, by daylight or after dark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub leather: Ramp,
    pub paper: Ramp,
    pub rule: Rgba,
    pub margin: Rgba,
    pub stitch: Ramp,
    pub brass: Ramp,
    pub cloth: Ramp,
    /// Writing on the page, and on the cover.
    pub ink: Rgba,
    pub cover_ink: Rgba,
    /// A sticky note's paper.
    pub note: Ramp,
}

/// The cream notebook of the daylight theme, and the charcoal one Desktop's notebook keeps after
/// dark. The leather is the same leather either way.
pub fn palette(dark: bool) -> Palette {
    let leather = Ramp::new(0x2f1c12, 0x4a2e1e, 0x5b3a27, 0x6e4a33, 0x87604a);
    let stitch = Ramp::new(0x8a7148, 0xb2955f, 0xcdb17a, 0xe2cc98, 0xf4e6c0);
    let brass = Ramp::new(0x6e4f1c, 0xa47a2c, 0xcf9f3e, 0xe8c25a, 0xfbe6a2);
    let cloth = Ramp::new(0x10281f, 0x173a2e, 0x1c4137, 0x285a4b, 0x3b7563);
    if dark {
        Palette {
            leather: Ramp::new(0x170e09, 0x24170f, 0x2b1f18, 0x3a2a20, 0x4c382b),
            paper: Ramp::new(0x111614, 0x181e1b, 0x1e2421, 0x262e2a, 0x313b36),
            rule: rgb(0x2a332f),
            margin: rgb(0x5e4038),
            stitch: Ramp::new(0x4a4030, 0x5e5240, 0x7a6a50, 0x948264, 0xb0a07e),
            brass: Ramp::new(0x3e2e14, 0x5e4620, 0x7e6230, 0x9a7c42, 0xb8a060),
            cloth: Ramp::new(0x0c1a15, 0x13261f, 0x1a3029, 0x234037, 0x2f5246),
            ink: rgb(0xe6ede4),
            cover_ink: rgb(0xd8ccb0),
            note: Ramp::new(0x302a1e, 0x3e3626, 0x4a4233, 0x585040, 0x6a624e),
        }
    } else {
        Palette {
            leather,
            paper: Ramp::new(0xc9b78e, 0xe8dab4, 0xf9f0d4, 0xfdf7e4, 0xfffcf2),
            rule: rgb(0xe9dbb2),
            margin: rgb(0xd99f89),
            stitch,
            brass,
            cloth,
            ink: rgb(0x23362f),
            cover_ink: rgb(0xf3e6c4),
            note: Ramp::new(0xb89a52, 0xe0c27c, 0xf6dc9a, 0xfbe8b6, 0xfff4d6),
        }
    }
}

/// A stud on the cover, the window's own buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Close,
    Shrink,
    Grow,
    Camera,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stud {
    pub at: Area,
    pub glyph: Glyph,
    /// Under the pointer, and pressed.
    pub hot: bool,
    pub down: bool,
}

/// A tab: stitched to the cover for the two modes, or standing up from the top of the notes page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tab {
    pub at: Area,
    pub open: bool,
    pub hot: bool,
}

/// Where everything is, in the notebook's own pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Spread {
    pub size: (i32, i32),
    /// The page the house is drawn on, and the page of notes.
    pub left: Area,
    pub right: Area,
    pub patch: Area,
    pub modes: Vec<Tab>,
    pub page_tabs: Vec<Tab>,
    pub studs: Vec<Stud>,
    /// Ruled lines on the notes page: how far down the first is, and how far apart.
    pub rules: (i32, i32),
    pub dark: bool,
}

/// The whole notebook, open.
pub fn paint(spread: &Spread) -> Canvas {
    let colours = palette(spread.dark);
    let (width, height) = spread.size;
    let mut canvas = Canvas::new(width.max(1) as u32, height.max(1) as u32);
    leather(&mut canvas, (0, 0, width, height), &colours.leather, 3);
    // The cover's edge: a lit rim along the top and left, shade along the bottom and right.
    let rim = colours.leather;
    paint::hline(&mut canvas, 0, 0, width, rim.edge);
    paint::vline(&mut canvas, 0, 0, height, rim.edge);
    paint::hline(&mut canvas, 0, height - 1, width, rim.edge);
    paint::vline(&mut canvas, width - 1, 0, height, rim.edge);
    paint::hline(&mut canvas, 1, 1, width - 2, rim.light);
    paint::vline(&mut canvas, 1, 1, height - 2, rim.light);
    paint::hline(&mut canvas, 1, height - 2, width - 2, rim.shadow);
    paint::vline(&mut canvas, width - 2, 1, height - 2, rim.shadow);
    stitching(&mut canvas, (4, 4, width - 8, height - 8), &colours.stitch);
    for corner in 0..4 {
        brass_corner(&mut canvas, (width, height), corner, &colours.brass);
    }
    // The pages lie a little proud of the cover, with a shadow under their far edges.
    for (page, outer_left) in [(spread.left, true), (spread.right, false)] {
        page_shadow(&mut canvas, page);
        paper(&mut canvas, page, &colours, outer_left);
    }
    binding(&mut canvas, spread.left, spread.right, &colours);
    ruled(&mut canvas, spread.right, spread.rules, &colours);
    for tab in &spread.page_tabs {
        page_tab(&mut canvas, tab, &colours);
    }
    patch(&mut canvas, spread.patch, &colours);
    for tab in &spread.modes {
        mode_tab(&mut canvas, tab, &colours);
    }
    for stud in &spread.studs {
        brass_stud(&mut canvas, stud, &colours);
    }
    canvas
}

/// Pebbled leather, a touch lighter towards the upper left.
fn leather(canvas: &mut Canvas, (x, y, w, h): Area, ramp: &Ramp, salt: u32) {
    let (cw, ch) = (canvas.width() as f32, canvas.height() as f32);
    for py in y..y + h {
        for px in x..x + w {
            let light = 1.0 - (px as f32 / cw + py as f32 / ch) / 2.0;
            let base = paint::mix(ramp.shadow, ramp.base, 0.55 + light * 0.45);
            // Pebbles two pixels across, each lit on its upper left and shaded on its lower right.
            let pebble = paint::noise(px.div_euclid(2), py.div_euclid(2), salt) % 100;
            let corner = (px.rem_euclid(2), py.rem_euclid(2));
            let tone = match (pebble, corner) {
                (0..=13, (0, 0)) => paint::mix(base, ramp.light, 0.6),
                (0..=13, (1, 1)) => paint::mix(base, ramp.shadow, 0.5),
                (14..=21, _) => paint::mix(base, ramp.shadow, 0.35),
                (22..=24, (0, _)) => paint::mix(base, ramp.light, 0.35),
                _ => base,
            };
            canvas.set(px, py, tone);
        }
    }
}

/// A line of saddle stitches round `area`: each stitch lit on its upper side, with the hole it
/// goes into shaded.
fn stitching(canvas: &mut Canvas, (x, y, w, h): Area, ramp: &Ramp) {
    let stitch = |canvas: &mut Canvas, sx: i32, sy: i32, across: bool| {
        if across {
            paint::hline(canvas, sx, sy, 3, ramp.base);
            paint::put(canvas, sx, sy, ramp.light);
            paint::put(canvas, sx + 3, sy, ramp.edge);
        } else {
            paint::vline(canvas, sx, sy, 3, ramp.base);
            paint::put(canvas, sx, sy, ramp.light);
            paint::put(canvas, sx, sy + 3, ramp.edge);
        }
    };
    let mut along = x + 2;
    while along + 3 < x + w - 1 {
        stitch(canvas, along, y, true);
        stitch(canvas, along, y + h - 1, true);
        along += 6;
    }
    let mut down = y + 2;
    while down + 3 < y + h - 1 {
        stitch(canvas, x, down, false);
        stitch(canvas, x + w - 1, down, false);
        down += 6;
    }
}

/// A brass plate folded over one corner of the cover, with a rivet.
fn brass_corner(canvas: &mut Canvas, (width, height): (i32, i32), corner: u8, ramp: &Ramp) {
    const ARM: i32 = 10;
    const THICK: i32 = 3;
    let (right, bottom) = (corner % 2 == 1, corner >= 2);
    let x = if right { width - ARM } else { 0 };
    let y = if bottom { height - ARM } else { 0 };
    let along_y = if bottom { height - THICK } else { 0 };
    let along_x = if right { width - THICK } else { 0 };
    paint::rect(canvas, x, along_y, ARM, THICK, ramp.base);
    paint::rect(canvas, along_x, y, THICK, ARM, ramp.base);
    // Lit along its top and left, shaded along its bottom and right.
    paint::hline(canvas, x, along_y, ARM, ramp.light);
    paint::vline(canvas, along_x, y, ARM, ramp.light);
    paint::hline(canvas, x, along_y + THICK - 1, ARM, ramp.shadow);
    paint::vline(canvas, along_x + THICK - 1, y, ARM, ramp.shadow);
    let (ex, ey) = (
        if right { width - ARM - 1 } else { ARM },
        if bottom { height - ARM - 1 } else { ARM },
    );
    paint::vline(canvas, ex, along_y, THICK, ramp.edge);
    paint::hline(canvas, along_x, ey, THICK, ramp.edge);
    let (rx, ry) = (
        if right { width - 2 } else { 1 },
        if bottom { height - 2 } else { 1 },
    );
    paint::put(canvas, rx, ry, ramp.shine);
}

/// The soft shadow a page casts on the cover, down and to the right.
fn page_shadow(canvas: &mut Canvas, (x, y, w, h): Area) {
    let shade = Rgba::new(20, 10, 6, 90);
    for py in y + 2..y + h + 2 {
        let tone = canvas.get(x + w, py);
        canvas.set(x + w, py, paint::over(shade, tone));
        let tone = canvas.get(x + w + 1, py);
        canvas.set(x + w + 1, py, paint::over(paint::faded(shade, 45), tone));
    }
    for px in x + 2..x + w + 2 {
        let tone = canvas.get(px, y + h);
        canvas.set(px, y + h, paint::over(shade, tone));
        let tone = canvas.get(px, y + h + 1);
        canvas.set(px, y + h + 1, paint::over(paint::faded(shade, 45), tone));
    }
}

/// A page: paper with a little grain in it, the edges of the pages under it showing at its outer
/// side, and shading where it curves down into the binding.
fn paper(canvas: &mut Canvas, (x, y, w, h): Area, colours: &Palette, outer_left: bool) {
    let ramp = colours.paper;
    for py in y..y + h {
        for px in x..x + w {
            // How near the binding: the page curves into it over its last few pixels.
            let from_binding = if outer_left { x + w - 1 - px } else { px - x };
            let curl = (1.0 - from_binding as f32 / 14.0).max(0.0);
            let fibre = paint::noise(px, py, 211) % 100;
            let base = match fibre {
                0..=3 => paint::mix(ramp.base, ramp.shadow, 0.35),
                4..=6 => ramp.light,
                _ => ramp.base,
            };
            let tone = paint::mix(base, ramp.shadow, curl * curl * 0.7);
            canvas.set(px, py, tone);
        }
    }
    // The page's edge in a darker shade of itself; at the outer side, the edges of the pages
    // beneath it.
    paint::hline(canvas, x, y, w, ramp.shadow);
    paint::hline(canvas, x, y + h - 1, w, ramp.edge);
    let outer = if outer_left { x } else { x + w - 1 };
    paint::vline(canvas, outer, y, h, ramp.edge);
    let step = if outer_left { -1 } else { 1 };
    for (depth, tone) in [(1, ramp.shadow), (2, ramp.edge)] {
        paint::vline(canvas, outer + step * depth, y + depth, h - depth, tone);
        paint::hline(canvas, x + depth * step.max(0), y + h - 1 + depth, w, tone);
    }
}

/// Where the two pages meet: the crease of the fold, with the binding's thread showing in it.
fn binding(canvas: &mut Canvas, left: Area, right: Area, colours: &Palette) {
    let fold = left.0 + left.2;
    let (top, bottom) = (
        left.1.max(right.1),
        (left.1 + left.3).min(right.1 + right.3),
    );
    for x in fold..right.0 {
        paint::vline(canvas, x, top, bottom - top, colours.paper.edge);
    }
    // The thread, in a stitch every so often down the fold.
    let mut y = top + 8;
    while y + 6 < bottom {
        paint::vline(canvas, fold, y, 4, colours.stitch.light);
        paint::put(canvas, fold, y + 4, colours.stitch.shadow);
        y += 24;
    }
}

/// The notes page's ruling, and its margin line.
fn ruled(canvas: &mut Canvas, (x, y, w, h): Area, (first, pitch): (i32, i32), colours: &Palette) {
    if pitch > 0 {
        let mut line = y + first;
        while line < y + h - 4 {
            for px in x + 3..x + w - 3 {
                if !paint::chance(px, line, 223, 18) {
                    canvas.set(px, line, colours.rule);
                }
            }
            line += pitch;
        }
    }
    let margin = x + 16;
    paint::vline(canvas, margin, y + 1, h - 2, colours.margin);
}

/// The cloth patch on the cover the house's name is written on: a woven green, stitched down.
fn patch(canvas: &mut Canvas, (x, y, w, h): Area, colours: &Palette) {
    if w <= 0 || h <= 0 {
        return;
    }
    let ramp = colours.cloth;
    // Its shadow on the leather.
    for py in y + 1..y + h + 1 {
        for px in x + 1..x + w + 1 {
            let tone = canvas.get(px, py);
            canvas.set(px, py, paint::over(Rgba::new(10, 5, 3, 110), tone));
        }
    }
    for py in y..y + h {
        for px in x..x + w {
            let weave = (px + py) % 2 == 0;
            let tone = if weave { ramp.base } else { ramp.light };
            let tone = if paint::chance(px, py, 227, 24) {
                ramp.shadow
            } else {
                tone
            };
            canvas.set(px, py, tone);
        }
    }
    stepped_edge(canvas, (x, y, w, h), ramp.edge);
    // Stitched down, a pixel in from its edge.
    let mut along = x + 3;
    while along + 2 < x + w - 2 {
        paint::hline(canvas, along, y + 2, 2, colours.stitch.base);
        paint::hline(canvas, along, y + h - 3, 2, colours.stitch.shadow);
        along += 4;
    }
}

/// A one-pixel outline round `area` with its corner pixels left out: the stepped corner every box
/// in Formiga's notebook has rather than a rounded one.
fn stepped_edge(canvas: &mut Canvas, (x, y, w, h): Area, colour: Rgba) {
    paint::hline(canvas, x + 1, y, w - 2, colour);
    paint::hline(canvas, x + 1, y + h - 1, w - 2, colour);
    paint::vline(canvas, x, y + 1, h - 2, colour);
    paint::vline(canvas, x + w - 1, y + 1, h - 2, colour);
    for (cx, cy) in [
        (x, y),
        (x + w - 1, y),
        (x, y + h - 1),
        (x + w - 1, y + h - 1),
    ] {
        canvas.set(cx, cy, Rgba::new(0, 0, 0, 0));
    }
}

/// A mode's tab on the cover: the open one is a slip of paper stitched on; the other is only its
/// lettering, pressed into the leather, with a faint outline where its slip would be when it is
/// pointed at.
fn mode_tab(canvas: &mut Canvas, tab: &Tab, colours: &Palette) {
    let (x, y, w, h) = tab.at;
    if tab.open {
        let ramp = colours.paper;
        paint::rect(canvas, x + 1, y + 1, w - 2, h - 2, ramp.base);
        paint::hline(canvas, x + 1, y + 1, w - 2, ramp.light);
        paint::hline(canvas, x + 1, y + h - 2, w - 2, ramp.shadow);
        stepped_edge(canvas, tab.at, ramp.edge);
        paint::put(canvas, x + 2, y + h / 2, colours.stitch.edge);
        paint::put(canvas, x + w - 3, y + h / 2, colours.stitch.edge);
    } else if tab.hot {
        stepped_edge(canvas, tab.at, colours.leather.light);
    }
}

/// A tab standing up from the top of the notes page: the open one is the page's own paper, joined
/// to it; the rest stand a little behind, a shade darker.
fn page_tab(canvas: &mut Canvas, tab: &Tab, colours: &Palette) {
    let (x, y, w, h) = tab.at;
    let ramp = colours.paper;
    let face = if tab.open {
        ramp.base
    } else if tab.hot {
        paint::mix(ramp.base, ramp.shadow, 0.3)
    } else {
        paint::mix(ramp.base, ramp.shadow, 0.6)
    };
    paint::rect(canvas, x + 1, y + 1, w - 2, h, face);
    paint::hline(canvas, x + 1, y, w - 2, ramp.edge);
    paint::vline(canvas, x, y + 1, h, ramp.edge);
    paint::vline(canvas, x + w - 1, y + 1, h, ramp.edge);
    paint::hline(
        canvas,
        x + 1,
        y + 1,
        w - 2,
        if tab.open { ramp.light } else { face },
    );
    if !tab.open {
        // Behind the page: its foot is hidden by the page's top edge.
        paint::hline(canvas, x, y + h, w, ramp.shadow);
    }
}

/// A round brass stud with its glyph struck into it.
fn brass_stud(canvas: &mut Canvas, stud: &Stud, colours: &Palette) {
    let (x, y, w, h) = stud.at;
    let ramp = colours.brass;
    let (cx, cy) = (x + w / 2, y + h / 2);
    let r = w.min(h) / 2;
    let face = if stud.down {
        ramp.shadow
    } else if stud.hot {
        ramp.light
    } else {
        ramp.base
    };
    for py in y..y + h {
        for px in x..x + w {
            let (dx, dy) = (px - cx, py - cy);
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let tone = if d2 > (r - 1) * (r - 1) {
                ramp.edge
            } else if dx + dy < -r / 2 && !stud.down {
                paint::mix(face, ramp.shine, 0.5)
            } else if dx + dy > r / 2 {
                paint::mix(face, ramp.shadow, 0.6)
            } else {
                face
            };
            canvas.set(px, py, tone);
        }
    }
    let mark = colours.leather.edge;
    match stud.glyph {
        Glyph::Close => {
            for d in -2..=2 {
                paint::put(canvas, cx + d, cy + d, mark);
                paint::put(canvas, cx + d, cy - d, mark);
            }
        }
        Glyph::Shrink => paint::hline(canvas, cx - 2, cy, 5, mark),
        Glyph::Grow => {
            paint::hline(canvas, cx - 2, cy - 2, 5, mark);
            paint::hline(canvas, cx - 2, cy + 2, 5, mark);
            paint::vline(canvas, cx - 2, cy - 2, 5, mark);
            paint::vline(canvas, cx + 2, cy - 2, 5, mark);
        }
        Glyph::Camera => {
            paint::hline(canvas, cx - 3, cy - 1, 7, mark);
            paint::hline(canvas, cx - 3, cy + 2, 7, mark);
            paint::vline(canvas, cx - 3, cy - 1, 4, mark);
            paint::vline(canvas, cx + 3, cy - 1, 4, mark);
            paint::hline(canvas, cx - 1, cy - 2, 3, mark);
            paint::put(canvas, cx, cy + 1, mark);
        }
    }
}

/// A sticky note, taped at its top: notes the window keeps for the owner, painted to sit over a
/// page.
pub fn note(size: (i32, i32), dark: bool) -> Canvas {
    let colours = palette(dark);
    let (w, h) = size;
    let mut canvas = Canvas::new((w + 2).max(1) as u32, (h + 2).max(1) as u32);
    let ramp = colours.note;
    for py in 2..h + 2 {
        for px in 2..w + 2 {
            canvas.set(px, py, Rgba::new(20, 10, 6, 70));
        }
    }
    for py in 0..h {
        for px in 0..w {
            let tone = if paint::chance(px, py, 229, 14) {
                ramp.shadow
            } else if py < 2 {
                ramp.light
            } else {
                ramp.base
            };
            canvas.set(px, py, tone);
        }
    }
    stepped_edge(&mut canvas, (0, 0, w, h), ramp.edge);
    // A strip of tape across its top.
    let tape = if dark {
        Rgba::new(150, 160, 140, 110)
    } else {
        Rgba::new(244, 238, 214, 200)
    };
    for py in -2..4 {
        for px in w / 2 - 12..w / 2 + 12 {
            let at = (px, py.max(0));
            let under = canvas.get(at.0, at.1);
            canvas.set(at.0, at.1, paint::over(tape, under));
        }
    }
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spread(dark: bool) -> Spread {
        Spread {
            size: (420, 300),
            left: (8, 26, 250, 250),
            right: (266, 26, 146, 250),
            patch: (40, 6, 90, 14),
            modes: vec![
                Tab {
                    at: (150, 7, 40, 12),
                    open: true,
                    hot: false,
                },
                Tab {
                    at: (194, 7, 50, 12),
                    open: false,
                    hot: true,
                },
            ],
            page_tabs: vec![Tab {
                at: (290, 20, 40, 7),
                open: true,
                hot: false,
            }],
            studs: vec![Stud {
                at: (8, 7, 11, 11),
                glyph: Glyph::Close,
                hot: false,
                down: false,
            }],
            rules: (24, 12),
            dark,
        }
    }

    #[test]
    fn the_notebook_covers_its_whole_window_and_its_pages_are_paper() {
        for dark in [false, true] {
            let spread = spread(dark);
            let canvas = paint(&spread);
            let colours = palette(dark);
            // Nothing is left clear but the stepped corners of a patch or a tab.
            let clear = canvas.pixels().iter().filter(|pixel| pixel.a == 0).count();
            assert!(clear <= 16, "{clear} clear pixels");
            // The middle of each page is paper, not leather.
            for (x, y, w, h) in [spread.left, spread.right] {
                let tone = canvas.get(x + w / 3, y + h / 2 + 1);
                let paperish = [
                    colours.paper.base,
                    colours.paper.light,
                    colours.rule,
                    paint::mix(colours.paper.base, colours.paper.shadow, 0.35),
                ];
                assert!(paperish.contains(&tone), "{dark}: {tone:?}");
            }
        }
    }
}
