//! The little signs over a resident's head that say what it is about: a heart, a note, a drift of
//! z's, a question. Each is a tiny picture outlined in a darker shade of itself, so it reads over
//! any wallpaper, and bobs gently unless motion is reduced.

use crate::paint::{self, rgb};
use formiga_art::{Canvas, Rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    Heart,
    Note,
    Sleep,
    Question,
    Bang,
    Sparkle,
    /// Thinking of something: a little cloud.
    Thought,
    Grumble,
}

fn picture(cue: Cue) -> (&'static [&'static str], Rgba) {
    match cue {
        Cue::Heart => (
            &[
                ".##.##.", "#######", "#######", ".#####.", "..###..", "...#...",
            ],
            rgb(0xe0566a),
        ),
        Cue::Note => (
            &[
                "...###", "...#.#", "...#.#", "...#..", ".###..", "####..", ".##...",
            ],
            rgb(0x5a6fb8),
        ),
        Cue::Sleep => (&["####", "..#.", ".#..", "####"], rgb(0x7f8fc4)),
        Cue::Question => (
            &[".###.", "#...#", "...#.", "..#..", ".....", "..#.."],
            rgb(0x5a8a5c),
        ),
        Cue::Bang => (&["#", "#", "#", "#", ".", "#"], rgb(0xd9813a)),
        Cue::Sparkle => (
            &[
                "...#...", "...#...", ".#.#.#.", "###.###", ".#.#.#.", "...#...", "...#...",
            ],
            rgb(0xe9b93a),
        ),
        Cue::Thought => (
            &[
                ".####..", "######.", "#######", ".#####.", "....#..", ".....#.",
            ],
            rgb(0xf4f0e8),
        ),
        Cue::Grumble => (&["#.#.#", ".#.#.", "#.#.#"], rgb(0x8a6a5a)),
    }
}

/// Draws `cue` centred on `x`, its foot at `y`, `seconds` into it.
/// How a cue `seconds` into its pose is moving: how far it bobs, and for a sleeper's, how far up
/// its second z has drifted and how clear it is. Whatever is the same, it is drawn the same.
pub fn motion(cue: Cue, seconds: f32, still: bool) -> (i32, Option<(i32, u8)>) {
    if still {
        return (0, None);
    }
    let bob = ((seconds * 3.0).sin() * 1.2).round() as i32;
    let drift = (cue == Cue::Sleep).then(|| {
        let drift = (seconds * 0.8).fract();
        ((drift * 4.0) as i32, (255.0 * (1.0 - drift)) as u8)
    });
    (bob, drift)
}

pub fn draw(canvas: &mut Canvas, cue: Cue, x: i32, y: i32, seconds: f32, still: bool) {
    let (rows, ink) = picture(cue);
    let height = rows.len() as i32;
    let width = rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
    let (bob, drift) = motion(cue, seconds, still);
    let mut glyph = Canvas::new((width + 2) as u32, (height + 2) as u32);
    for (dy, row) in rows.iter().enumerate() {
        for (dx, code) in row.bytes().enumerate() {
            if code == b'#' {
                glyph.set(dx as i32 + 1, dy as i32 + 1, ink);
            }
        }
    }
    // Ringed in a darker shade of itself, then lit along its top.
    let source = glyph.clone();
    for gy in 0..glyph.height() as i32 {
        for gx in 0..glyph.width() as i32 {
            if source.get(gx, gy).a > 0 {
                continue;
            }
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(ox, oy)| source.get(gx + ox, gy + oy).a > 0);
            if near {
                glyph.set(gx, gy, paint::darker(ink, 0.6));
            }
        }
    }
    let (left, top) = (
        x - glyph.width() as i32 / 2,
        y - glyph.height() as i32 + bob,
    );
    paint::blit(canvas, &glyph, left, top);
    if let Some((up, clear)) = drift {
        // A second, smaller z drifting up after the first.
        let (zx, zy) = (left + width + 1, top - 2 - up);
        for (dx, dy) in [(0, 0), (1, 0), (1, 1), (0, 2), (1, 2)] {
            paint::put(canvas, zx + dx, zy + dy, paint::faded(ink, clear));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_draws_something_in_its_own_place() {
        for cue in [
            Cue::Heart,
            Cue::Note,
            Cue::Sleep,
            Cue::Question,
            Cue::Bang,
            Cue::Sparkle,
            Cue::Thought,
            Cue::Grumble,
        ] {
            let mut canvas = Canvas::new(32, 32);
            draw(&mut canvas, cue, 16, 20, 0.0, true);
            let (x0, y0, x1, y1) = canvas.alpha_bounds().expect("a cue draws");
            assert!(x0 >= 8 && x1 <= 24 && y0 >= 8 && y1 <= 20, "{cue:?}");
        }
    }
}
