//! Formiga Battle's keepsakes, a trophy and a medal for each of its four cups, drawn at a find's
//! size from the house's own ramps. Every trophy is the same little brass cup, stood on what its
//! cup's arena has to hand: a wooden plinth from the Backyard, a mossy stump from Woodland Glade,
//! a millstone from the Millpond and a piece of driftwood from Sandy Cove. Every medal is the same
//! brass disc, hung on its cup's ribbon: green, russet, blue and sandy yellow. Desktop says the
//! same in each keepsake's description.

use super::ramps::{BARK, BRASS, CLAY, CORNFLOWER, DRIFTWOOD, FERN, MOSS, OAK, WICKER};
use crate::paint::{self, Ramp};
use formiga_art::Canvas;
use formiga_core::BattleKeepsake;

/// The keepsake's picture.
pub fn picture(keepsake: BattleKeepsake) -> Canvas {
    use BattleKeepsake::*;
    match keepsake {
        BackyardCupTrophy => trophy(Stand::Plinth),
        WoodlandCupTrophy => trophy(Stand::Stump),
        MillpondCupTrophy => trophy(Stand::Millstone),
        CoveCupTrophy => trophy(Stand::Driftwood),
        BackyardCupMedal => medal(FERN),
        WoodlandCupMedal => medal(CLAY),
        MillpondCupMedal => medal(CORNFLOWER),
        CoveCupMedal => medal(WICKER),
    }
}

/// What a trophy stands on.
#[derive(Clone, Copy)]
enum Stand {
    Plinth,
    Stump,
    Millstone,
    Driftwood,
}

/// A little brass cup with two handles on a short stem, on its stand.
fn trophy(stand: Stand) -> Canvas {
    let mut canvas = Canvas::new(11, 13);
    // The bowl, widest at the rim, lit from the upper left.
    for (y, (x0, width)) in [(2, 7), (2, 7), (3, 5), (3, 5), (4, 3)]
        .into_iter()
        .enumerate()
    {
        let y = y as i32 + 1;
        paint::hline(&mut canvas, x0, y, width, BRASS.base);
        paint::put(&mut canvas, x0, y, BRASS.light);
        paint::put(&mut canvas, x0 + width - 1, y, BRASS.shadow);
    }
    paint::hline(&mut canvas, 2, 0, 7, BRASS.light);
    paint::put(&mut canvas, 3, 2, BRASS.shine);
    // The handles.
    for x in [0, 10] {
        paint::vline(&mut canvas, x, 1, 2, BRASS.shadow);
    }
    paint::put(&mut canvas, 1, 3, BRASS.shadow);
    paint::put(&mut canvas, 9, 3, BRASS.shadow);
    // The stem and its foot.
    paint::vline(&mut canvas, 5, 6, 2, BRASS.shadow);
    paint::hline(&mut canvas, 4, 8, 3, BRASS.base);
    paint::put(&mut canvas, 4, 8, BRASS.light);
    match stand {
        Stand::Plinth => {
            paint::rect(&mut canvas, 2, 9, 7, 4, OAK.base);
            paint::hline(&mut canvas, 2, 9, 7, OAK.light);
            paint::vline(&mut canvas, 8, 10, 3, OAK.shadow);
            paint::hline(&mut canvas, 4, 11, 3, BRASS.light);
        }
        Stand::Stump => {
            paint::rect(&mut canvas, 2, 10, 7, 3, BARK.base);
            paint::vline(&mut canvas, 4, 10, 3, BARK.shadow);
            paint::vline(&mut canvas, 7, 11, 2, BARK.shadow);
            paint::hline(&mut canvas, 2, 9, 7, MOSS.base);
            paint::put(&mut canvas, 3, 9, MOSS.light);
            paint::put(&mut canvas, 1, 12, MOSS.shadow);
            paint::put(&mut canvas, 9, 12, MOSS.shadow);
        }
        Stand::Millstone => {
            // Millstone grit: the driftwood's grey, a good deal darker.
            let stone = |color| paint::darker(color, 0.25);
            paint::rect(&mut canvas, 1, 9, 9, 4, stone(DRIFTWOOD.base));
            paint::hline(&mut canvas, 1, 9, 9, stone(DRIFTWOOD.light));
            paint::hline(&mut canvas, 1, 12, 9, stone(DRIFTWOOD.shadow));
            // The grooves of its dressing.
            for x in [3, 7] {
                paint::put(&mut canvas, x, 10, stone(DRIFTWOOD.shadow));
                paint::put(&mut canvas, x - 1, 11, stone(DRIFTWOOD.shadow));
            }
        }
        Stand::Driftwood => {
            paint::hline(&mut canvas, 1, 9, 8, DRIFTWOOD.light);
            paint::rect(&mut canvas, 0, 10, 11, 2, DRIFTWOOD.base);
            paint::hline(&mut canvas, 2, 12, 8, DRIFTWOOD.shadow);
            paint::put(&mut canvas, 3, 10, DRIFTWOOD.shine);
            paint::put(&mut canvas, 8, 11, DRIFTWOOD.shadow);
        }
    }
    edged(&mut canvas);
    canvas
}

/// A brass disc on a ribbon of its cup's colour, the ribbon's two halves crossing above it.
fn medal(ribbon: Ramp) -> Canvas {
    let mut canvas = Canvas::new(9, 13);
    for y in 0..6 {
        let step = y / 2;
        paint::put(&mut canvas, 1 + step, y, ribbon.light);
        paint::put(&mut canvas, 2 + step, y, ribbon.base);
        paint::put(&mut canvas, 6 - step, y, ribbon.base);
        paint::put(&mut canvas, 7 - step, y, ribbon.shadow);
    }
    paint::ellipse(&mut canvas, 4, 9, 3, 3, BRASS.base);
    for (x, y) in [(3, 7), (2, 7), (2, 8), (1, 9)] {
        paint::put(&mut canvas, x, y, BRASS.light);
    }
    for (x, y) in [(7, 9), (6, 10), (6, 11), (5, 11), (4, 12)] {
        paint::put(&mut canvas, x, y, BRASS.shadow);
    }
    paint::put(&mut canvas, 3, 8, BRASS.shine);
    paint::put(&mut canvas, 4, 9, BRASS.light);
    edged(&mut canvas);
    canvas
}

/// Every edge a shade darker, as the household's own keepsakes are.
fn edged(canvas: &mut Canvas) {
    paint::outline_inside(canvas, |color| {
        if color.a < 200 {
            color
        } else {
            paint::darker(color, 0.6)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keepsake_is_drawn_and_no_two_alike() {
        let pictures: Vec<Canvas> = BattleKeepsake::ALL.into_iter().map(picture).collect();
        for (index, canvas) in pictures.iter().enumerate() {
            assert!(
                canvas.alpha_bounds().is_some(),
                "{:?}",
                BattleKeepsake::ALL[index]
            );
            for other in &pictures[..index] {
                assert!(canvas != other, "{:?}", BattleKeepsake::ALL[index]);
            }
        }
    }
}
