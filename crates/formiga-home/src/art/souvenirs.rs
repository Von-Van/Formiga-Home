//! Formiga Hill's souvenirs as a house keeps them, each its own way rather than all on velvet:
//! the ribbon tied on a hook or folded on a napkin, the daisy pressed under glass, the penny in a
//! dish, the acorn in a thimble, the feather in a bud vase or pinned up, the marble on a brass
//! stand, and the ticket tacked up or held in a clip. The souvenir itself is Hill's own
//! picture, untouched; only what keeps it is drawn here.

use super::Sprite;
use super::displays::pinned;
use super::ramps::{BERRY, BRASS, CORNFLOWER, LINEN, SAGE};
use crate::paint::{self, rgb, rgba};
use formiga_art::{Canvas, Rgba};
use formiga_core::Souvenir;

/// A thing's soft shadow on the surface it stands on.
fn foot_shadow(canvas: &mut Canvas, x: i32, y: i32, half: i32) {
    paint::ellipse(canvas, x, y, half, 1, rgba(0x2a1a14, 60));
}

/// A thing's shadow on the wallpaper, a pixel down and to the right of it.
fn wall_shadow(canvas: &mut Canvas, picture: &Canvas, x: i32, y: i32) {
    for py in 0..picture.height() as i32 {
        for px in 0..picture.width() as i32 {
            if picture.get(px, py).a > 0 {
                paint::put(canvas, x + px + 1, y + py + 1, rgba(0x3a2a24, 60));
            }
        }
    }
}

/// Hung on a wall.
pub fn hung(souvenir: Souvenir, icon: &Canvas) -> Sprite {
    match souvenir {
        Souvenir::PicnicRibbon => on_hook(icon),
        Souvenir::PressedDaisy => under_glass(icon, false),
        Souvenir::FairTicket => tacked(icon),
        _ => pinned(icon),
    }
}

/// Set down on a table, a shelf, or in a case.
pub fn set_down(souvenir: Souvenir, icon: &Canvas) -> Sprite {
    match souvenir {
        Souvenir::PicnicRibbon => on_napkin(icon),
        Souvenir::PressedDaisy => under_glass(icon, true),
        Souvenir::WellPenny => in_dish(icon),
        Souvenir::OakAcorn => in_thimble(icon),
        Souvenir::SwingFeather => in_bud_vase(icon),
        Souvenir::ChestMarble => on_stand(icon),
        Souvenir::FairTicket => in_clip(icon),
    }
}

/// The bow, tied by a loop of thread to a brass hook.
fn on_hook(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(10, 11);
    wall_shadow(&mut canvas, icon, 1, 3);
    paint::blit(&mut canvas, icon, 1, 3);
    paint::put(&mut canvas, 4, 0, BRASS.shadow);
    paint::put(&mut canvas, 4, 1, BRASS.light);
    paint::put(&mut canvas, 4, 2, LINEN.base);
    paint::put(&mut canvas, 4, 3, LINEN.base);
    Sprite {
        canvas,
        anchor: (4, 5),
        over: None,
        lids: Vec::new(),
    }
}

/// Pressed under glass in a slim brass frame, hung, or stood on a shelf.
fn under_glass(icon: &Canvas, standing: bool) -> Sprite {
    let (w, h) = (11, 11);
    let mut canvas = Canvas::new(w as u32 + 1, h as u32 + if standing { 2 } else { 1 });
    if standing {
        foot_shadow(&mut canvas, w / 2, h, w / 2);
    } else {
        paint::rect(&mut canvas, 1, 1, w, h, rgba(0x3a2a24, 55));
    }
    paint::rect(&mut canvas, 0, 0, w, h, BRASS.base);
    paint::hline(&mut canvas, 0, 0, w, BRASS.light);
    paint::vline(&mut canvas, 0, 0, h, BRASS.light);
    paint::hline(&mut canvas, 0, h - 1, w, BRASS.edge);
    paint::vline(&mut canvas, w - 1, 0, h, BRASS.edge);
    // Pressed on sage paper, so the white of its petals shows.
    paint::rect(&mut canvas, 1, 1, w - 2, h - 2, SAGE.light);
    paint::hline(&mut canvas, 1, 1, w - 2, SAGE.base);
    paint::blit(&mut canvas, icon, 2, 2);
    // The glass over it, and a glint across its corner.
    paint::rect(&mut canvas, 1, 1, w - 2, h - 2, rgba(0xd8f0f2, 45));
    paint::put(&mut canvas, 7, 2, rgba(0xffffff, 170));
    paint::put(&mut canvas, 8, 3, rgba(0xffffff, 170));
    let anchor = if standing {
        (w / 2, h - 1)
    } else {
        (w / 2, h / 2)
    };
    Sprite {
        canvas,
        anchor,
        over: None,
        lids: Vec::new(),
    }
}

/// The ticket stub tacked up by a red tack in its corner.
fn tacked(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(10, 10);
    wall_shadow(&mut canvas, icon, 1, 1);
    paint::blit(&mut canvas, icon, 1, 1);
    paint::put(&mut canvas, 2, 2, BERRY.light);
    paint::put(&mut canvas, 2, 3, BERRY.edge);
    Sprite {
        canvas,
        anchor: (4, 4),
        over: None,
        lids: Vec::new(),
    }
}

/// The bow on a folded linen napkin.
fn on_napkin(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(11, 10);
    foot_shadow(&mut canvas, 5, 9, 5);
    paint::hline(&mut canvas, 2, 6, 8, LINEN.shine);
    paint::hline(&mut canvas, 1, 7, 9, LINEN.light);
    paint::hline(&mut canvas, 1, 8, 8, LINEN.base);
    paint::blit(&mut canvas, icon, 2, 0);
    Sprite {
        canvas,
        anchor: (5, 8),
        over: None,
        lids: Vec::new(),
    }
}

/// The penny in a little glazed dish with a blue band round it.
fn in_dish(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(11, 10);
    foot_shadow(&mut canvas, 5, 9, 5);
    paint::ellipse(&mut canvas, 5, 7, 5, 2, LINEN.shine);
    paint::ellipse(&mut canvas, 5, 7, 4, 1, LINEN.base);
    paint::blit(&mut canvas, icon, 2, 2);
    // The dish's near rim, in front of the penny's foot.
    paint::hline(&mut canvas, 1, 8, 9, LINEN.shine);
    paint::hline(&mut canvas, 2, 9, 7, CORNFLOWER.base);
    paint::put(&mut canvas, 0, 7, LINEN.base);
    paint::put(&mut canvas, 10, 7, LINEN.shadow);
    Sprite {
        canvas,
        anchor: (5, 9),
        over: None,
        lids: Vec::new(),
    }
}

const SILVER: [Rgba; 4] = [
    Rgba::new(0xe4, 0xe8, 0xec, 255),
    Rgba::new(0xb8, 0xbe, 0xc6, 255),
    Rgba::new(0x86, 0x8c, 0x96, 255),
    Rgba::new(0x9a, 0xa0, 0xa8, 255),
];

/// The acorn sat in a silver thimble, dimpled all over.
fn in_thimble(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(9, 12);
    foot_shadow(&mut canvas, 4, 11, 3);
    paint::blit(&mut canvas, icon, 1, 1);
    let [light, base, shade, dimple] = SILVER;
    paint::hline(&mut canvas, 2, 6, 5, light);
    for y in 7..=10 {
        paint::put(&mut canvas, 2, y, light);
        paint::hline(&mut canvas, 3, y, 3, base);
        paint::put(&mut canvas, 6, y, shade);
    }
    for (x, y) in [(3, 8), (5, 8), (4, 9), (3, 10), (5, 10)] {
        paint::put(&mut canvas, x, y, dimple);
    }
    Sprite {
        canvas,
        anchor: (4, 10),
        over: None,
        lids: Vec::new(),
    }
}

/// The feather stood in a slim glass bud vase, its quill seen through the glass.
fn in_bud_vase(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(10, 13);
    foot_shadow(&mut canvas, 2, 12, 3);
    paint::blit(&mut canvas, icon, 2, 0);
    paint::vline(&mut canvas, 2, 6, 5, rgb(0x7a8590));
    let glass = rgba(0xd8f0f2, 120);
    let edge = rgb(0x8fb4b8);
    for (y, half) in [(6, 1), (7, 1), (8, 2), (9, 2), (10, 2), (11, 1)] {
        paint::hline(&mut canvas, 2 - half, y, half * 2 + 1, glass);
        paint::put(&mut canvas, 2 - half, y, edge);
        paint::put(&mut canvas, 2 + half, y, edge);
    }
    paint::hline(&mut canvas, 1, 11, 3, edge);
    paint::put(&mut canvas, 1, 9, rgba(0xffffff, 200));
    Sprite {
        canvas,
        anchor: (2, 11),
        over: None,
        lids: Vec::new(),
    }
}

/// The marble on a little brass stand: a cup, a stem and a foot.
fn on_stand(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(9, 11);
    foot_shadow(&mut canvas, 4, 10, 3);
    paint::blit(&mut canvas, icon, 1, 0);
    paint::hline(&mut canvas, 2, 6, 5, BRASS.light);
    paint::hline(&mut canvas, 3, 7, 3, BRASS.base);
    paint::put(&mut canvas, 4, 8, BRASS.shadow);
    paint::hline(&mut canvas, 2, 9, 5, BRASS.base);
    paint::put(&mut canvas, 2, 9, BRASS.light);
    paint::put(&mut canvas, 6, 9, BRASS.edge);
    Sprite {
        canvas,
        anchor: (4, 9),
        over: None,
        lids: Vec::new(),
    }
}

/// The ticket held up in a brass clip on a little foot.
fn in_clip(icon: &Canvas) -> Sprite {
    let mut canvas = Canvas::new(9, 11);
    foot_shadow(&mut canvas, 4, 10, 3);
    paint::blit(&mut canvas, icon, 1, 0);
    paint::put(&mut canvas, 3, 6, BRASS.light);
    paint::put(&mut canvas, 5, 6, BRASS.shadow);
    paint::vline(&mut canvas, 4, 6, 3, BRASS.shadow);
    paint::hline(&mut canvas, 2, 9, 5, BRASS.base);
    paint::put(&mut canvas, 2, 9, BRASS.light);
    Sprite {
        canvas,
        anchor: (4, 9),
        over: None,
        lids: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_souvenir_is_kept_its_own_way_and_stands_no_taller_than_a_shelf_allows() {
        let mut seen = Vec::new();
        for souvenir in Souvenir::ALL {
            let mut icon = Canvas::new(formiga_art::SOUVENIR_ICON, formiga_art::SOUVENIR_ICON);
            formiga_art::draw_souvenir(&mut icon, souvenir, 0, 0);
            let kept = set_down(souvenir, &icon);
            let top = (0..kept.canvas.height() as i32)
                .find(|&y| (0..kept.canvas.width() as i32).any(|x| kept.canvas.get(x, y).a > 0))
                .unwrap();
            assert!(kept.anchor.1 - top <= 13, "{souvenir:?} stands too tall");
            let bytes = kept.canvas.rgba_bytes();
            assert!(!seen.contains(&bytes), "{souvenir:?} is kept like another");
            seen.push(bytes);
        }
    }
}
