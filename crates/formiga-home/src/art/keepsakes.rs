//! The household's keepsakes, each drawn at a find's size from the house's own ramps: a
//! postcard, a jar of jam, a pressed flower, a rosette, a painted pebble, a little one's drawing
//! and a framed photo. A drawing or a photo shows whoever is in it in the colours they had when it
//! was made; without them, a drawing is in crayon and a photo is of the room alone.

use super::ramps::{BERRY, BRASS, CORNFLOWER, FERN, LINEN, OAK, ROSE};
use crate::paint::{self, rgb, rgba};
use formiga_art::{Canvas, Rgba};
use formiga_home_contract::{Ink, Memento, MementoKind};

/// A keepsake's picture, with whoever is in it.
pub fn picture(memento: &Memento) -> Canvas {
    draw(memento.kind, &memento.inks)
}

/// A keepsake of a kind, with nobody in particular in it.
pub fn plain(kind: MementoKind) -> Option<Canvas> {
    (kind != MementoKind::Unknown).then(|| draw(kind, &[]))
}

fn draw(kind: MementoKind, inks: &[Ink]) -> Canvas {
    match kind {
        MementoKind::Postcard => postcard(),
        MementoKind::JamJar => jam_jar(),
        MementoKind::PressedFlower => pressed_flower(),
        MementoKind::Rosette => rosette(),
        MementoKind::Pebble => pebble(),
        MementoKind::Drawing => drawing(inks),
        MementoKind::Photo | MementoKind::Unknown => photo(inks),
    }
}

/// Someone's colours, as they are drawn small.
#[derive(Clone, Copy)]
struct Likeness {
    outline: Rgba,
    deep: Rgba,
    body: Rgba,
    light: Rgba,
}

impl Likeness {
    fn of(ink: &Ink) -> Self {
        let color = |[r, g, b]: [u8; 3]| Rgba::new(r, g, b, 255);
        Self {
            outline: color(ink.outline),
            deep: color(ink.deep),
            body: color(ink.body),
            light: color(ink.light),
        }
    }
}

/// How a little one draws anyone it has not got the colours of: an orange crayon.
const CRAYON: Likeness = Likeness {
    outline: Rgba::new(0x9a, 0x4a, 0x2a, 255),
    deep: Rgba::new(0xd0, 0x72, 0x3c, 255),
    body: Rgba::new(0xf0, 0x9a, 0x5a, 255),
    light: Rgba::new(0xf8, 0xc0, 0x88, 255),
};

/// A postcard from somewhere a friend has been: a hill under a sky on the left, a stamp and a
/// line or two on the right.
fn postcard() -> Canvas {
    let mut canvas = Canvas::new(12, 8);
    paint::rect(&mut canvas, 0, 0, 12, 8, LINEN.light);
    paint::rect(&mut canvas, 1, 1, 5, 3, CORNFLOWER.light);
    paint::rect(&mut canvas, 1, 4, 5, 3, FERN.base);
    paint::hline(&mut canvas, 1, 4, 5, FERN.light);
    paint::put(&mut canvas, 2, 2, BRASS.shine);
    paint::vline(&mut canvas, 7, 2, 5, LINEN.base);
    paint::rect(&mut canvas, 9, 1, 2, 2, BERRY.base);
    paint::put(&mut canvas, 9, 1, BERRY.light);
    for y in [4, 6] {
        paint::hline(&mut canvas, 8, y, 3, rgb(0x8c7c62));
    }
    paper_edged(&mut canvas);
    canvas
}

/// A jar of jam, its lid in gingham and a label across it.
fn jam_jar() -> Canvas {
    let mut canvas = Canvas::new(7, 9);
    for x in 1..6 {
        let check = if x % 2 == 0 { BERRY.base } else { LINEN.shine };
        paint::put(&mut canvas, x, 0, check);
    }
    for x in 0..7 {
        let check = if x % 2 == 1 { BERRY.base } else { LINEN.shine };
        paint::put(&mut canvas, x, 1, check);
    }
    paint::hline(&mut canvas, 1, 2, 5, rgb(0xd8eef0));
    paint::rect(&mut canvas, 0, 3, 7, 5, BERRY.shadow);
    paint::hline(&mut canvas, 1, 8, 5, BERRY.edge);
    paint::vline(&mut canvas, 1, 3, 4, BERRY.light);
    paint::rect(&mut canvas, 1, 5, 5, 2, LINEN.light);
    paint::put(&mut canvas, 3, 5, BERRY.base);
    edged(&mut canvas);
    canvas
}

/// A flower pressed flat on a scrap of paper, taped at its corners.
fn pressed_flower() -> Canvas {
    let mut canvas = Canvas::new(8, 10);
    paint::rect(&mut canvas, 0, 0, 8, 10, rgb(0xfbf6ea));
    paint::vline(&mut canvas, 4, 4, 5, FERN.base);
    paint::put(&mut canvas, 3, 6, FERN.light);
    paint::put(&mut canvas, 5, 7, FERN.light);
    for (x, y) in [(3, 3), (5, 3), (4, 2), (4, 4)] {
        paint::put(&mut canvas, x, y, ROSE.base);
    }
    for (x, y) in [(3, 2), (5, 2), (3, 4), (5, 4)] {
        paint::put(&mut canvas, x, y, ROSE.light);
    }
    paint::put(&mut canvas, 4, 3, BRASS.light);
    paper_edged(&mut canvas);
    for x in [0, 7] {
        paint::put(&mut canvas, x, 0, rgba(0xe8dcc0, 220));
    }
    canvas
}

/// A prize rosette: a pleated ring of ribbon round a gold middle, and two tails.
fn rosette() -> Canvas {
    let mut canvas = Canvas::new(9, 12);
    let centre = (4.0_f32, 4.0_f32);
    for y in 0..9 {
        for x in 0..9 {
            let (dx, dy) = (x as f32 - centre.0, y as f32 - centre.1);
            let far = (dx * dx + dy * dy).sqrt();
            if far > 4.4 {
                continue;
            }
            let color = if far < 2.2 {
                if (x, y) == (3, 3) {
                    BRASS.shine
                } else {
                    BRASS.light
                }
            } else {
                // A pleat every eighth of the way round.
                let sector =
                    ((dy.atan2(dx) + std::f32::consts::PI) / (std::f32::consts::PI / 4.0)) as i32;
                if sector % 2 == 0 {
                    BERRY.base
                } else {
                    BERRY.light
                }
            };
            paint::put(&mut canvas, x, y, color);
        }
    }
    for (x, y) in [(3, 9), (2, 10), (5, 9), (6, 10)] {
        paint::put(&mut canvas, x, y, BERRY.base);
        paint::put(&mut canvas, x, y - 1, BERRY.shadow);
    }
    paint::put(&mut canvas, 1, 11, BERRY.shadow);
    paint::put(&mut canvas, 7, 11, BERRY.shadow);
    edged(&mut canvas);
    canvas
}

/// A smooth pebble with three dots painted on it.
fn pebble() -> Canvas {
    let mut canvas = Canvas::new(7, 5);
    for y in 0..5 {
        for x in 0..7 {
            let (dx, dy) = ((x as f32 - 3.0) / 3.5, (y as f32 - 2.0) / 2.5);
            if dx * dx + dy * dy > 1.0 {
                continue;
            }
            let lit = dx + dy;
            let tone = if lit < -0.5 {
                rgb(0xcac4ba)
            } else if lit < 0.5 {
                rgb(0xa8a198)
            } else {
                rgb(0x878078)
            };
            paint::put(&mut canvas, x, y, tone);
        }
    }
    paint::put(&mut canvas, 2, 2, BERRY.light);
    paint::put(&mut canvas, 3, 2, BRASS.light);
    paint::put(&mut canvas, 4, 2, CORNFLOWER.light);
    edged(&mut canvas);
    canvas
}

/// A little one's drawing on a sheet with a corner turned down: a sun, the grass, and whoever it
/// is of, in their own colours.
fn drawing(inks: &[Ink]) -> Canvas {
    let mut canvas = Canvas::new(11, 9);
    let paper = rgb(0xfdfbf6);
    paint::rect(&mut canvas, 0, 0, 11, 9, paper);
    paint::rect(&mut canvas, 1, 1, 2, 2, BRASS.light);
    paint::put(&mut canvas, 3, 1, BRASS.base);
    paint::put(&mut canvas, 1, 3, BRASS.base);
    paint::hline(&mut canvas, 1, 7, 9, FERN.light);
    let first = inks.first().map_or(CRAYON, Likeness::of);
    let mut figure = |x: i32, y: i32, who: Likeness| {
        paint::put(&mut canvas, x, y, who.outline);
        paint::put(&mut canvas, x + 2, y, who.outline);
        paint::hline(&mut canvas, x, y + 1, 3, who.body);
        paint::hline(&mut canvas, x - 1, y + 2, 5, who.body);
        paint::hline(&mut canvas, x - 1, y + 3, 5, who.deep);
        paint::put(&mut canvas, x, y + 1, who.light);
        paint::put(&mut canvas, x, y + 2, who.outline);
        paint::put(&mut canvas, x + 2, y + 2, who.outline);
    };
    figure(6, 3, first);
    if let Some(second) = inks.get(1) {
        figure(2, 4, Likeness::of(second));
    }
    paper_edged(&mut canvas);
    // The turned-down corner.
    canvas.set(10, 0, Rgba::new(0, 0, 0, 0));
    paint::put(&mut canvas, 9, 0, rgb(0xd8d0c0));
    paint::put(&mut canvas, 10, 1, rgb(0xd8d0c0));
    canvas
}

/// A photo in an oak frame: a corner of the room, and whoever was in it, the first in front.
fn photo(inks: &[Ink]) -> Canvas {
    let (w, h) = (13, 10);
    let mut canvas = Canvas::new(w as u32, h as u32);
    paint::rect(&mut canvas, 0, 0, w, h, OAK.base);
    paint::hline(&mut canvas, 0, 0, w, OAK.light);
    paint::vline(&mut canvas, 0, 0, h, OAK.light);
    paint::hline(&mut canvas, 0, h - 1, w, OAK.shadow);
    paint::vline(&mut canvas, w - 1, 0, h, OAK.shadow);
    paint::rect(&mut canvas, 1, 1, w - 2, 5, rgb(0xe8dcc4));
    paint::rect(&mut canvas, 1, 6, w - 2, 3, rgb(0xc9935e));
    let people: Vec<Likeness> = inks.iter().take(6).map(Likeness::of).collect();
    let (front, back) = people.split_at(people.len().min(3));
    // Each three wide with a pixel between, centred across the photo.
    let row = |count: usize| -> Vec<i32> {
        let span = count as i32 * 4 - 1;
        let left = 1 + (w - 2 - span) / 2;
        (0..count as i32).map(|index| left + index * 4).collect()
    };
    let figure = |canvas: &mut Canvas, x: i32, y: i32, who: &Likeness| {
        paint::put(canvas, x, y, who.deep);
        paint::put(canvas, x + 2, y, who.deep);
        paint::hline(canvas, x, y + 1, 3, who.body);
        paint::put(canvas, x, y + 1, who.light);
        paint::hline(canvas, x, y + 2, 3, who.body);
        paint::put(canvas, x + 1, y + 2, who.outline);
        paint::hline(canvas, x, y + 3, 3, who.deep);
    };
    for (who, x) in back.iter().zip(row(back.len())) {
        figure(&mut canvas, x + 2, 2, who);
    }
    for (who, x) in front.iter().zip(row(front.len())) {
        figure(&mut canvas, x, 5, who);
    }
    paint::put(&mut canvas, 2, 1, rgba(0xffffff, 150));
    canvas
}

/// Every edge in a darker shade of its own colour.
fn edged(canvas: &mut Canvas) {
    edged_by(canvas, 0.6);
}

/// Paper's edge, a shade darker only: a sheet, not a board.
fn paper_edged(canvas: &mut Canvas) {
    edged_by(canvas, 0.3);
}

fn edged_by(canvas: &mut Canvas, dark: f32) {
    paint::outline_inside(canvas, |color| {
        if color.a < 200 {
            color
        } else {
            paint::darker(color, dark)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keepsake_has_a_picture_about_a_find_s_size() {
        for kind in [
            MementoKind::Postcard,
            MementoKind::JamJar,
            MementoKind::PressedFlower,
            MementoKind::Rosette,
            MementoKind::Pebble,
            MementoKind::Drawing,
            MementoKind::Photo,
        ] {
            let picture = plain(kind).unwrap();
            assert!(picture.width() <= 14 && picture.height() <= 13, "{kind:?}");
            assert!(picture.alpha_bounds().is_some(), "{kind:?}");
        }
        assert!(plain(MementoKind::Unknown).is_none());
    }

    #[test]
    fn a_photo_shows_whoever_was_in_it_in_their_own_colours() {
        let ink = Ink {
            outline: [10, 20, 30],
            deep: [40, 50, 60],
            body: [200, 30, 40],
            light: [220, 220, 30],
            accent: [0, 0, 0],
        };
        let photo = photo(&[ink]);
        let has = |r, g, b| {
            (0..photo.height() as i32).any(|y| {
                (0..photo.width() as i32).any(|x| photo.get(x, y) == Rgba::new(r, g, b, 255))
            })
        };
        assert!(has(200, 30, 40) && has(220, 220, 30));
    }
}
