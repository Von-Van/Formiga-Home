//! Pictures of the house: a photo the owner saves, with one framed for the drawer; and, for review,
//! one of the whole window, after which it closes.

use super::*;

impl HomeApp {
    /// For review only: open on `page` of the arranging notes, or living in the house if none,
    /// and after `at` seconds save a picture of the window to `path` and close.
    pub fn snap(&mut self, path: PathBuf, at: f32, page: Option<&str>, zoom: i32) {
        self.snap = Some((path, at, false));
        self.snap_zoom = zoom;
        let page = match page {
            Some("finds") => Some(Drawer::Finds),
            Some("furniture") => Some(Drawer::Furniture),
            Some("rooms") => Some(Drawer::Room),
            Some("journal") => {
                self.live_page = Drawer::Journal;
                None
            }
            _ => None,
        };
        if let Some(page) = page {
            self.drawer = page;
            self.set_mode(Mode::Arrange);
        }
    }

    /// Ask for the picture when it is time, and save it when it comes.
    pub(super) fn take_snap(&mut self, ctx: &egui::Context) {
        let Some((path, at, asked)) = self.snap.clone() else {
            return;
        };
        // Once the page has been laid out once, so the zoom knows what fits.
        if self.snap_zoom > 0 && self.zoom.fit > 0.0 {
            for _ in 0..self.snap_zoom {
                self.zoom.step(1, egui::Vec2::ZERO);
            }
            self.snap_zoom = 0;
        }
        if !asked && self.now() >= at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.snap = Some((path.clone(), at, true));
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let mut canvas = Canvas::new(width as u32, height as u32);
            for (index, pixel) in image.pixels.iter().enumerate() {
                let [r, g, b, a] = pixel.to_srgba_unmultiplied();
                canvas.set(
                    (index % width) as i32,
                    (index / width) as i32,
                    formiga_art::Rgba::new(r, g, b, a),
                );
            }
            if let Err(error) = crate::write_png(&path, &canvas, 1) {
                eprintln!("formiga-home: {error:#}");
            }
            self.snap = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// The house as a photo: everyone where they are, on the table-top light, and none of the
    /// window's own marks.
    pub(super) fn photo(&mut self) -> Canvas {
        let now = self.now();
        let house = self.house_as_seen();
        let overlay = Overlay {
            lamps_off: self.life.lamps_off().to_vec(),
            backdrop: true,
            daylight: self.daylight(),
            ..Overlay::default()
        };
        self.scene.compose(
            &house,
            &self.household.snapshot,
            &mut self.life.actors,
            now,
            &overlay,
        )
    }

    /// Ask where to save a photo of the room, and save it there, three times the size.
    pub(super) fn save_photo(&mut self) {
        let canvas = self.photo();
        let name = format!("{}.png", self.household.house_name());
        let chosen = rfd::FileDialog::new()
            .set_title("Save a picture of the room")
            .set_file_name(&name)
            .add_filter("PNG image", &["png"])
            .save_file();
        let Some(path) = chosen else { return };
        match crate::write_png(&path, &canvas, 3) {
            Ok(()) => {
                // And one framed for the house, of whoever was in it.
                let in_it: Vec<Id> = self
                    .life
                    .actors
                    .iter()
                    .filter(|actor| !actor.hidden && self.house.room_of_point(actor.pos).is_some())
                    .map(|actor| actor.id)
                    .collect();
                if self
                    .make_keepsake(MementoKind::Photo, None, in_it)
                    .is_some()
                {
                    self.say("Saved a picture of the room, and framed one for the drawer.");
                } else {
                    self.say("Saved a picture of the room.");
                }
            }
            Err(error) => self.say(format!("The picture could not be saved: {error}")),
        }
    }
}
