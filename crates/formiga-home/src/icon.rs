//! Home's icon: a little room, drawn pixel by pixel at 32 by 32 the way Home draws its rooms, and
//! scaled up by whole pixels for every size the window, the macOS bundle and the Windows
//! installer ask for.

use crate::paint::{self, rgb};
use formiga_art::Canvas;

/// The picture, at its own size.
fn picture() -> Canvas {
    let mut canvas = Canvas::new(32, 32);
    let wall = rgb(0xb9cfa0);
    let leaf = rgb(0x7f9a6a);
    let floor = rgb(0xc98d52);
    let board = rgb(0xa86a3c);
    let edge = rgb(0x6b3f24);
    // The two far walls, the right one in the light and the left in its own shade.
    paint::polygon(
        &mut canvas,
        &[(16.0, 3.0), (30.0, 10.0), (30.0, 20.0), (16.0, 13.0)],
        wall,
    );
    paint::polygon(
        &mut canvas,
        &[(16.0, 3.0), (2.0, 10.0), (2.0, 20.0), (16.0, 13.0)],
        paint::mix(wall, rgb(0x5a4a5c), 0.18),
    );
    for (x, y) in [(20, 9), (25, 11), (7, 11), (11, 9)] {
        paint::put(&mut canvas, x, y, leaf);
        paint::put(&mut canvas, x, y + 1, leaf);
    }
    // The floor, its boards, and the cut edge in front.
    paint::polygon(
        &mut canvas,
        &[(16.0, 13.0), (30.0, 20.0), (16.0, 27.0), (2.0, 20.0)],
        floor,
    );
    for step in [16.0_f32, 19.0, 22.0] {
        paint::line(
            &mut canvas,
            (5, (step + 1.5) as i32),
            (16, (step - 4.0) as i32),
            board,
        );
    }
    paint::polygon(
        &mut canvas,
        &[(2.0, 20.0), (16.0, 27.0), (16.0, 30.0), (2.0, 23.0)],
        edge,
    );
    paint::polygon(
        &mut canvas,
        &[(30.0, 20.0), (16.0, 27.0), (16.0, 30.0), (30.0, 23.0)],
        paint::darker(edge, 0.3),
    );
    // A little bed against the right wall, and a resident on the floor.
    paint::polygon(
        &mut canvas,
        &[(19.0, 15.0), (25.0, 18.0), (25.0, 16.0), (19.0, 13.0)],
        rgb(0xd98c9c),
    );
    paint::polygon(
        &mut canvas,
        &[(19.0, 13.0), (21.0, 12.0), (27.0, 15.0), (25.0, 16.0)],
        rgb(0xecb0bb),
    );
    paint::ellipse(&mut canvas, 13, 20, 3, 3, rgb(0x8fae7e));
    paint::put(&mut canvas, 12, 19, rgb(0x2a2433));
    paint::put(&mut canvas, 14, 19, rgb(0x2a2433));
    canvas
}

/// The icon at `size` pixels square: the picture scaled up by whole pixels and centred, or, for
/// the few sizes smaller than it, every other pixel of it.
pub fn at(size: u32) -> Canvas {
    let small = picture();
    let mut canvas = Canvas::new(size, size);
    if size < 32 {
        let step = 32.0 / size as f32;
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                let (sx, sy) = ((x as f32 * step) as i32, (y as f32 * step) as i32);
                canvas.set(x, y, small.get(sx, sy));
            }
        }
        return canvas;
    }
    let scale = (size / 32) as i32;
    let offset = (size as i32 - 32 * scale) / 2;
    for y in 0..32 * scale {
        for x in 0..32 * scale {
            canvas.set(x + offset, y + offset, small.get(x / scale, y / scale));
        }
    }
    canvas
}

/// A canvas as PNG bytes.
pub fn png(canvas: &Canvas) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, canvas.width(), canvas.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("a PNG header writes to memory");
        writer
            .write_image_data(&canvas.rgba_bytes())
            .expect("a PNG writes to memory");
    }
    bytes
}

/// A macOS `.icns`: PNGs at 128, 256, 512 and 1024 pixels.
pub fn icns() -> Vec<u8> {
    let entries: Vec<(&[u8; 4], Vec<u8>)> = [
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
    ]
    .into_iter()
    .map(|(kind, size)| (kind, png(&at(size))))
    .collect();
    let length = 8 + entries
        .iter()
        .map(|(_, data)| 8 + data.len())
        .sum::<usize>();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(b"icns");
    bytes.extend_from_slice(&(length as u32).to_be_bytes());
    for (kind, data) in entries {
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
        bytes.extend_from_slice(&data);
    }
    bytes
}

/// A Windows `.ico`: PNGs at 16, 32, 48 and 256 pixels.
pub fn ico() -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = [16, 32, 48, 256]
        .into_iter()
        .map(|size| (size, png(&at(size))))
        .collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0, 0, 1, 0]);
    bytes.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, data) in &images {
        // A side of 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        bytes.extend_from_slice(&[side, side, 0, 0]);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in images {
        bytes.extend_from_slice(&data);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_is_the_same_picture_at_every_size() {
        let small = at(32);
        let big = at(256);
        for (x, y) in [(13, 20), (16, 5), (20, 14), (3, 21)] {
            assert_eq!(small.get(x, y), big.get(x * 8 + 3, y * 8 + 3), "({x}, {y})");
        }
        let tiny = at(16);
        for (x, y) in [(6, 10), (8, 2)] {
            assert_eq!(tiny.get(x, y), small.get(x * 2, y * 2));
        }
        assert!(small.get(0, 0).a == 0, "the corners stay clear");
    }

    #[test]
    fn the_icon_files_say_what_they_hold() {
        let icns = icns();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        let ico = ico();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 4);
    }
}
