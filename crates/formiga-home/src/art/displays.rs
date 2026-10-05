//! What the colony shows, drawn where it is shown. Every thing has a picture already — a find is
//! drawn by `formiga-art` in its colony's inks, a souvenir as Formiga Hill draws it — so every
//! thing can be shown as itself, sitting on a shelf or pinned to a wall, or on a card or in a frame
//! when it is somewhere it would not sit as itself. A thing this build has no picture for is shown
//! on a card with a question mark, never left out.

use super::Sprite;
use super::ramps::{BRASS, LINEN, OAK, VELVET};
use crate::paint::{self, rgb, rgba};
use crate::room::{Place, Showing};
use formiga_art::{Canvas, TRINKET_CELL, draw_souvenir, draw_trinket};
use formiga_core::{Souvenir, TRINKET_VARIANTS};
use formiga_home_contract::{DisplayItem, DisplaySource};

/// The thing's own picture, cropped to what it covers.
pub fn icon(item: &DisplayItem) -> Canvas {
    match (&item.source, item.ink) {
        (DisplaySource::DesktopFind { variant }, Some(ink)) if *variant < TRINKET_VARIANTS => {
            let mut cell = Canvas::new(TRINKET_CELL, TRINKET_CELL);
            draw_trinket(
                &mut cell,
                ink.to_art(),
                *variant,
                formiga_art::TRINKET_FRAME_REST,
                0,
                0,
            );
            crop(&cell)
        }
        (DisplaySource::HillSouvenir { id }, _) => match Souvenir::from_id(id) {
            Some(souvenir) => {
                let mut tile = Canvas::new(formiga_art::SOUVENIR_ICON, formiga_art::SOUVENIR_ICON);
                draw_souvenir(&mut tile, souvenir, 0, 0);
                tile
            }
            None => unknown_icon(),
        },
        _ => unknown_icon(),
    }
}

/// Whether the thing is one of Formiga Hill's souvenirs, which sit on velvet as they do there.
fn is_souvenir(item: &DisplayItem) -> bool {
    matches!(&item.source, DisplaySource::HillSouvenir { id } if Souvenir::from_id(id).is_some())
}

fn crop(canvas: &Canvas) -> Canvas {
    let Some((x0, y0, x1, y1)) = canvas.alpha_bounds() else {
        return unknown_icon();
    };
    let mut out = Canvas::new(x1 - x0 + 1, y1 - y0 + 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            out.set(
                (x - x0) as i32,
                (y - y0) as i32,
                canvas.get(x as i32, y as i32),
            );
        }
    }
    out
}

/// A question mark, for a thing a newer Desktop has and this build cannot draw.
fn unknown_icon() -> Canvas {
    const ROWS: [&str; 7] = [
        ".###.", "#...#", "....#", "..##.", "..#..", ".....", "..#..",
    ];
    let mut canvas = Canvas::new(5, 7);
    for (y, row) in ROWS.iter().enumerate() {
        for (x, code) in row.bytes().enumerate() {
            if code == b'#' {
                canvas.set(x as i32, y as i32, rgb(0x6b4a3a));
            }
        }
    }
    canvas
}

/// The thing as it is shown in a place of this kind. Its anchor is where it sits: the middle of
/// its foot on a surface or the floor, and its middle on a wall.
pub fn sprite(item: &DisplayItem, place: Place, showing: Showing) -> Sprite {
    let icon = icon(item);
    match (place, showing) {
        (Place::Wall, Showing::Itself) if is_souvenir(item) => framed(&icon, true),
        (Place::Wall, Showing::Itself) => pinned(&icon),
        (Place::Wall, Showing::Card) => framed(&icon, false),
        (_, Showing::Card) => card(&icon, is_souvenir(item)),
        (_, Showing::Itself) if is_souvenir(item) => on_velvet(&icon),
        (Place::Floor, Showing::Itself) => standing(&icon, true),
        (_, Showing::Itself) => standing(&icon, false),
    }
}

/// As itself, standing on a surface or the floor, with a little shadow under it.
fn standing(icon: &Canvas, on_floor: bool) -> Sprite {
    let (w, h) = (icon.width() as i32, icon.height() as i32);
    let mut canvas = Canvas::new((w + 6) as u32, (h + 4) as u32);
    let foot = (w / 2 + 3, h + 1);
    let shadow = if on_floor {
        (w / 2 + 1, 2, 70)
    } else {
        (w / 2, 1, 60)
    };
    paint::ellipse(
        &mut canvas,
        foot.0,
        foot.1,
        shadow.0,
        shadow.1,
        rgba(0x2a1a14, shadow.2),
    );
    paint::blit(&mut canvas, icon, 3, 1);
    Sprite {
        canvas,
        anchor: foot,
        over: None,
    }
}

/// As itself, pinned flat to a wall, a pin at its top and its shadow on the wallpaper behind.
fn pinned(icon: &Canvas) -> Sprite {
    let (w, h) = (icon.width() as i32, icon.height() as i32);
    let mut canvas = Canvas::new((w + 4) as u32, (h + 5) as u32);
    for y in 0..h {
        for x in 0..w {
            if icon.get(x, y).a > 0 {
                paint::put(&mut canvas, x + 2, y + 3, rgba(0x3a2a24, 60));
            }
        }
    }
    paint::blit(&mut canvas, icon, 1, 2);
    let pin = (w / 2 + 1, 1);
    paint::put(&mut canvas, pin.0, pin.1, BRASS.shadow);
    paint::put(&mut canvas, pin.0, pin.1 + 1, BRASS.base);
    paint::put(&mut canvas, pin.0 - 1, pin.1, BRASS.light);
    Sprite {
        canvas,
        anchor: (w / 2 + 1, h / 2 + 2),
        over: None,
    }
}

/// In a little frame on the wall: oak round a cream mount, or round velvet for a souvenir.
fn framed(icon: &Canvas, velvet: bool) -> Sprite {
    let (w, h) = (icon.width() as i32, icon.height() as i32);
    let (fw, fh) = (w + 8, h + 8);
    let mut canvas = Canvas::new((fw + 1) as u32, (fh + 1) as u32);
    // A shadow on the wall to the lower right.
    paint::rect(&mut canvas, 1, 1, fw, fh, rgba(0x3a2a24, 55));
    let frame = if velvet { BRASS } else { OAK };
    paint::rect(&mut canvas, 0, 0, fw, fh, frame.base);
    paint::hline(&mut canvas, 0, 0, fw, frame.light);
    paint::vline(&mut canvas, 0, 0, fh, frame.light);
    paint::hline(&mut canvas, 0, fh - 1, fw, frame.edge);
    paint::vline(&mut canvas, fw - 1, 0, fh, frame.edge);
    let mount = if velvet { VELVET.base } else { LINEN.light };
    paint::rect(&mut canvas, 2, 2, fw - 4, fh - 4, mount);
    paint::hline(
        &mut canvas,
        2,
        2,
        fw - 4,
        if velvet { VELVET.shadow } else { LINEN.base },
    );
    paint::blit(&mut canvas, icon, 4, 4);
    Sprite {
        canvas,
        anchor: (fw / 2, fh / 2),
        over: None,
    }
}

/// On a little card standing on its own foot: a specimen card, the thing pictured on it. A
/// souvenir's card is mounted in velvet, the way Formiga Hill shows it, so a pale one still reads.
fn card(icon: &Canvas, velvet: bool) -> Sprite {
    let (w, h) = (icon.width() as i32, icon.height() as i32);
    let (cw, ch) = (w + 6, h + 6);
    let mut canvas = Canvas::new((cw + 4) as u32, (ch + 4) as u32);
    paint::ellipse(
        &mut canvas,
        cw / 2 + 2,
        ch + 2,
        cw / 2,
        1,
        rgba(0x2a1a14, 60),
    );
    paint::rect(&mut canvas, 2, 1, cw, ch, LINEN.light);
    paint::hline(&mut canvas, 2, 1, cw, LINEN.shine);
    paint::vline(&mut canvas, 2 + cw - 1, 1, ch, LINEN.shadow);
    paint::hline(&mut canvas, 2, ch, cw, LINEN.shadow);
    let mount = if velvet { VELVET.base } else { rgb(0xf7f0de) };
    paint::rect(&mut canvas, 4, 3, cw - 4, ch - 4, mount);
    paint::blit(&mut canvas, icon, 5, 4);
    paint::outline_inside(&mut canvas, |color| {
        if color.a < 200 {
            color
        } else {
            paint::darker(color, 0.45)
        }
    });
    Sprite {
        canvas,
        anchor: (cw / 2 + 2, ch + 2),
        over: None,
    }
}

/// A souvenir sat on a little velvet cushion, the way Formiga Hill keeps it in its case.
fn on_velvet(icon: &Canvas) -> Sprite {
    let (w, h) = (icon.width() as i32, icon.height() as i32);
    let mut canvas = Canvas::new((w + 8) as u32, (h + 7) as u32);
    let centre = (w / 2 + 4, h + 3);
    paint::ellipse(
        &mut canvas,
        centre.0,
        centre.1 + 1,
        w / 2 + 3,
        2,
        rgba(0x2a1a14, 60),
    );
    paint::ellipse(&mut canvas, centre.0, centre.1, w / 2 + 3, 2, VELVET.base);
    paint::hline(
        &mut canvas,
        centre.0 - w / 2 - 1,
        centre.1 - 1,
        w + 2,
        VELVET.light,
    );
    paint::blit(&mut canvas, icon, 4, 1);
    Sprite {
        canvas,
        anchor: (centre.0, centre.1 + 1),
        over: None,
    }
}

/// Every thing the sample colony has, shown every way it can be, for review: `--render-finds`.
pub fn sheet(items: &[DisplayItem]) -> Canvas {
    let cell = (36, 36);
    let places = [Place::Top, Place::Shelf, Place::Wall, Place::Floor];
    let columns = places.len() as i32 + 1;
    let mut sheet = Canvas::new(
        (cell.0 * columns) as u32,
        (cell.1 * items.len() as i32) as u32,
    );
    for (row, item) in items.iter().enumerate() {
        let top = row as i32 * cell.1;
        for column in 0..columns {
            let tone = if (row as i32 + column) % 2 == 0 {
                0xeadfcf
            } else {
                0xdfd2bf
            };
            paint::rect(&mut sheet, column * cell.0, top, cell.0, cell.1, rgb(tone));
        }
        let card = sprite(item, Place::Top, Showing::Card);
        let (ox, oy) = card.origin((cell.0 / 2, top + cell.1 - 6));
        paint::blit(&mut sheet, &card.canvas, ox, oy);
        for (index, place) in places.into_iter().enumerate() {
            let Some(showing) = crate::room::showing(item, place) else {
                continue;
            };
            let drawn = sprite(item, place, showing);
            let column = index as i32 + 1;
            let at = if place == Place::Wall {
                (column * cell.0 + cell.0 / 2, top + cell.1 / 2)
            } else {
                (column * cell.0 + cell.0 / 2, top + cell.1 - 6)
            };
            let (ox, oy) = drawn.origin(at);
            paint::blit(&mut sheet, &drawn.canvas, ox, oy);
        }
    }
    sheet
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_home_contract::sample;

    #[test]
    fn every_thing_in_the_sample_has_a_picture_every_way_it_can_be_shown() {
        for item in &sample::snapshot().inventory {
            for place in [Place::Top, Place::Shelf, Place::Wall, Place::Floor] {
                if let Some(showing) = crate::room::showing(item, place) {
                    let drawn = sprite(item, place, showing);
                    assert!(
                        drawn.canvas.alpha_bounds().is_some(),
                        "{} at {place:?}",
                        item.id
                    );
                }
            }
        }
    }

    #[test]
    fn every_find_and_souvenir_desktop_has_can_be_shown_and_has_a_picture_every_way() {
        let mut save = sample::colony();
        let finder = save.creatures[0].id;
        for variant in 0..TRINKET_VARIANTS {
            save.companion
                .remember_discovery(variant, finder, "Mochi".to_owned(), sample::MADE);
        }
        save.trips.souvenirs = Souvenir::ALL
            .map(|souvenir| formiga_core::SouvenirRecord {
                souvenir,
                brought_home_at_utc: sample::MADE,
            })
            .to_vec();
        let snapshot = formiga_home_contract::project_household(
            &save,
            sample::keeper(&save),
            &[],
            formiga_home_contract::SessionId::parse(sample::SESSION).unwrap(),
            sample::MADE,
            "test",
        )
        .unwrap();
        assert_eq!(
            snapshot.inventory.len(),
            usize::from(TRINKET_VARIANTS) + Souvenir::ALL.len()
        );
        for item in &snapshot.inventory {
            let mut shown = 0;
            for place in [Place::Top, Place::Shelf, Place::Wall, Place::Floor] {
                if let Some(showing) = crate::room::showing(item, place) {
                    let drawn = sprite(item, place, showing);
                    assert!(
                        drawn.canvas.alpha_bounds().is_some(),
                        "{} at {place:?}",
                        item.id
                    );
                    shown += 1;
                }
            }
            assert!(shown >= 3, "{} can be shown only {shown} ways", item.id);
            assert!(
                icon(item).alpha_bounds().is_some(),
                "{} has no picture",
                item.id
            );
        }
    }

    #[test]
    fn a_thing_this_build_cannot_draw_still_has_its_card() {
        let mut stranger = sample::snapshot().inventory[0].clone();
        stranger.source = DisplaySource::Unknown;
        stranger.ink = None;
        let drawn = sprite(&stranger, Place::Top, Showing::Card);
        assert!(drawn.canvas.alpha_bounds().is_some());
    }
}
