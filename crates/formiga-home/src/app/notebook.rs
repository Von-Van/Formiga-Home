//! The notebook round the window: where everything goes for the window's size, the cover painted
//! to fit it, and what the cover's own controls do. Its studs close, shrink and grow the window
//! and take a photo; its two stitched tabs change between living in the house and arranging it;
//! and the leather moves the window wherever it is dragged, since the system's own frame is gone.
//!
//! Everything is laid out in the notebook's own pixels, so that what is written on the cover and
//! the pages lands exactly on what was painted for it.

use super::*;
use crate::art::notebook::{self, Area, Glyph, Spread, Stud, Tab};

/// The notebook's pixel, in points: as near a point and a half as whole screen pixels allow, so
/// it is as crisp as the house drawn on its page.
pub(super) fn unit(pixels_per_point: f32) -> f32 {
    (1.5 * pixels_per_point).round().max(2.0) / pixels_per_point
}

/// The interface's colours, for what is written in the notebook rather than painted on it.
pub(super) mod ink {
    use eframe::egui::Color32;

    pub fn page(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(230, 237, 228)
        } else {
            Color32::from_rgb(35, 54, 47)
        }
    }

    /// Headings, and the accent the notebook is built round.
    pub fn forest(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(159, 196, 168)
        } else {
            Color32::from_rgb(28, 65, 55)
        }
    }

    /// Present, but not asking for attention.
    pub fn muted(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(150, 162, 148)
        } else {
            Color32::from_rgb(110, 119, 108)
        }
    }

    /// Whatever is chosen.
    pub fn mint(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(47, 81, 71)
        } else {
            Color32::from_rgb(174, 221, 186)
        }
    }

    /// The notebook's outlines: plum by daylight, as the creatures are outlined, sage after dark.
    pub fn line(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(85, 100, 90)
        } else {
            Color32::from_rgb(59, 43, 58)
        }
    }

    /// The face of a card, a shade away from the page.
    pub fn card(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(38, 46, 42)
        } else {
            Color32::from_rgb(255, 248, 228)
        }
    }

    /// The red of a rubber stamp: what is shown in this house.
    pub fn stamp(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(212, 140, 118)
        } else {
            Color32::from_rgb(181, 84, 63)
        }
    }

    /// Lettering on the leather.
    pub fn cover(_dark: bool) -> Color32 {
        Color32::from_rgb(243, 230, 196)
    }

    /// Lettering pressed into the leather.
    pub fn deboss(_dark: bool) -> Color32 {
        Color32::from_rgb(178, 149, 112)
    }
}

/// The size every printed label in the notebook is set at, as Desktop's notebook sets them.
pub(super) const LABEL: f32 = 10.5;

/// A printed label: small spaced capitals.
pub(super) fn label_job(text: &str, size: f32, color: egui::Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(size),
            color,
            extra_letter_spacing: (size * 0.08).round().max(0.5),
            ..Default::default()
        },
    );
    job
}

/// A section's printed label on a page.
pub(super) fn kicker(ui: &mut egui::Ui, text: &str) {
    let dark = ui.visuals().dark_mode;
    ui.label(label_job(text, LABEL, ink::forest(dark)));
}

/// The painted cover, kept until the window changes size or a control on it changes.
#[derive(Default)]
pub(super) struct Chrome {
    spread: Option<Spread>,
    texture: Option<egui::TextureHandle>,
    /// Which stud and tab were under the pointer or pressed last frame, for painting this one.
    hot: Option<egui::Id>,
    down: Option<egui::Id>,
}

/// Where everything is this frame, in points.
pub(super) struct Layout {
    pub unit: f32,
    pub window: egui::Rect,
    pub strip: egui::Rect,
    pub status: egui::Rect,
    pub left: egui::Rect,
    pub right: egui::Rect,
    /// Where the notes are written on the right-hand page: inside its margin.
    pub notes: egui::Rect,
    pub patch: egui::Rect,
    pub modes: [(Mode, egui::Rect); 2],
    pub page_tabs: Vec<(Drawer, egui::Rect)>,
    pub studs: Vec<(Glyph, egui::Rect)>,
    pub dark: bool,
}

/// The pages of the notes when arranging, and what their tabs say.
pub(super) const PAGES: [(Drawer, &str); 3] = [
    (Drawer::Finds, "Found things"),
    (Drawer::Furniture, "Furniture"),
    (Drawer::Room, "Rooms"),
];

/// The pages of the notes when living in the house.
pub(super) const LIVE_PAGES: [(Drawer, &str); 2] = [
    (Drawer::Household, "Household"),
    (Drawer::Journal, "Journal"),
];

/// The pages of the notes in a mode.
fn pages(mode: Mode) -> &'static [(Drawer, &'static str)] {
    match mode {
        Mode::Live => &LIVE_PAGES,
        Mode::Arrange => &PAGES,
    }
}

/// The window's own studs on the cover, at the end of the strip the system would put them.
fn studs() -> Vec<Glyph> {
    if cfg!(target_os = "macos") {
        vec![Glyph::Close, Glyph::Shrink, Glyph::Grow]
    } else {
        vec![Glyph::Shrink, Glyph::Grow, Glyph::Close]
    }
}

/// The notebook laid out for a window, in its own pixels and in points.
pub(super) fn lay_out(ui: &egui::Ui, house: &str, mode: Mode) -> (Layout, Spread) {
    let ctx = ui.ctx();
    let dark = ui.visuals().dark_mode;
    let window = ctx.content_rect();
    let unit = unit(ctx.pixels_per_point());
    let to_points = |(x, y, w, h): Area| {
        egui::Rect::from_min_size(
            window.min + egui::vec2(x as f32, y as f32) * unit,
            egui::vec2(w as f32, h as f32) * unit,
        )
    };
    let width_of = |job: egui::text::LayoutJob| {
        let galley = ui.painter().layout_job(job);
        (galley.size().x / unit).ceil() as i32
    };
    let (w, h) = (
        (window.width() / unit).floor() as i32,
        (window.height() / unit).floor() as i32,
    );
    const RIM: i32 = 9;
    const STRIP: i32 = 23;
    const FOOT: i32 = 20;
    let right_width = (w as f32 * 0.31).clamp(150.0, 204.0) as i32;
    let (top, bottom) = (STRIP, h - FOOT);
    let right = (w - RIM - right_width, top, right_width, bottom - top);
    let left = (RIM, top, right.0 - 1 - RIM, bottom - top);

    // The strip along the top: studs at one end, then the house's name on its patch, then the
    // mode tabs; the camera at the other end.
    let glyphs = studs();
    let stud_size = 12;
    let studs_width = glyphs.len() as i32 * (stud_size + 3);
    let mac = cfg!(target_os = "macos");
    let stud_left = if mac {
        RIM + 3
    } else {
        w - RIM - 3 - studs_width + 3
    };
    let mut stud_areas: Vec<(Glyph, Area)> = glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            (
                *glyph,
                (
                    stud_left + index as i32 * (stud_size + 3),
                    4,
                    stud_size,
                    stud_size,
                ),
            )
        })
        .collect();
    let camera_left = if mac {
        w - RIM - 3 - stud_size
    } else {
        stud_left - 8 - stud_size
    };
    stud_areas.push((Glyph::Camera, (camera_left, 4, stud_size, stud_size)));
    let name_width = width_of(label_job(house, 12.0, ink::cover(dark)));
    let patch_left = if mac {
        stud_left + studs_width + 6
    } else {
        RIM + 6
    };
    let patch = (patch_left, 5, name_width + 14, 15);
    let mut along = patch.0 + patch.2 + 10;
    let modes: Vec<(Mode, Area)> = [(Mode::Live, "Live"), (Mode::Arrange, "Arrange")]
        .into_iter()
        .map(|(which, name)| {
            let wide = width_of(label_job(name, LABEL, ink::page(dark))) + 10;
            let at = (along, 7, wide, 12);
            along += wide + 3;
            (which, at)
        })
        .collect();
    // The notes page's tabs stand up from its top edge, below the studs, so that neither
    // stands in the other's way.
    let mut page_tabs = Vec::new();
    let mut along = right.0 + 6;
    for &(drawer, name) in pages(mode) {
        let wide = width_of(label_job(name, LABEL, ink::page(dark))) + 6;
        page_tabs.push((drawer, (along, top - 7, wide, 7)));
        along += wide + 1;
    }
    let spread = Spread {
        size: (w + 1, h + 1),
        left,
        right,
        patch,
        modes: Vec::new(),
        page_tabs: Vec::new(),
        studs: Vec::new(),
        rules: (22, 12),
        dark,
    };
    let notes = to_points(right);
    let notes = egui::Rect::from_min_max(
        notes.min + egui::vec2(21.0 * unit, 8.0 * unit),
        notes.max - egui::vec2(6.0 * unit, 5.0 * unit),
    );
    let layout = Layout {
        unit,
        window,
        strip: to_points((0, 0, w + 1, STRIP)),
        status: to_points((RIM + 3, bottom + 3, w - 2 * RIM - 6, FOOT - 9)),
        left: to_points(left),
        right: to_points(right),
        notes,
        patch: to_points(patch),
        modes: [
            (modes[0].0, to_points(modes[0].1)),
            (modes[1].0, to_points(modes[1].1)),
        ],
        page_tabs: page_tabs
            .iter()
            .map(|(drawer, at)| (*drawer, to_points(*at)))
            .collect(),
        studs: stud_areas
            .iter()
            .map(|(glyph, at)| (*glyph, to_points(*at)))
            .collect(),
        dark,
    };
    let mut spread = spread;
    spread.modes = modes
        .iter()
        .map(|(which, at)| Tab {
            at: *at,
            open: *which == mode,
            hot: false,
        })
        .collect();
    spread.page_tabs = page_tabs
        .iter()
        .map(|(_, at)| Tab {
            at: *at,
            open: false,
            hot: false,
        })
        .collect();
    spread.studs = stud_areas
        .iter()
        .map(|(glyph, at)| Stud {
            at: *at,
            glyph: *glyph,
            hot: false,
            down: false,
        })
        .collect();
    (layout, spread)
}

/// The id a stud or a tab answers to.
pub(super) fn stud_id(glyph: Glyph) -> egui::Id {
    egui::Id::new(("stud", glyph as u8))
}

pub(super) fn mode_id(mode: Mode) -> egui::Id {
    egui::Id::new(("mode", mode as u8))
}

pub(super) fn page_id(drawer: Drawer) -> egui::Id {
    egui::Id::new(("page", drawer as u8))
}

impl HomeApp {
    /// The notes page turned to, in the mode the house is in.
    fn page(&self) -> Drawer {
        match self.mode {
            Mode::Live => self.live_page,
            Mode::Arrange => self.drawer,
        }
    }

    /// The cover, the pages, and everything on the cover: painted, lettered, and answering.
    pub(super) fn notebook(&mut self, ui: &mut egui::Ui) -> Layout {
        let ctx = ui.ctx().clone();
        let (layout, mut spread) = lay_out(ui, &self.household.house_name(), self.mode);
        // Last frame's pointer, for which stud or tab is lit.
        let (hot, down) = (self.chrome.hot, self.chrome.down);
        for (stud, (glyph, _)) in spread.studs.iter_mut().zip(&layout.studs) {
            stud.hot = hot == Some(stud_id(*glyph));
            stud.down = down == Some(stud_id(*glyph));
        }
        for (tab, (mode, _)) in spread.modes.iter_mut().zip(&layout.modes) {
            tab.hot = hot == Some(mode_id(*mode));
        }
        let page = self.page();
        for (tab, (drawer, _)) in spread.page_tabs.iter_mut().zip(&layout.page_tabs) {
            tab.open = *drawer == page;
            tab.hot = hot == Some(page_id(*drawer));
        }
        if self.chrome.spread.as_ref() != Some(&spread) {
            let canvas = notebook::paint(&spread);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas.width() as usize, canvas.height() as usize],
                &canvas.rgba_bytes(),
            );
            match &mut self.chrome.texture {
                Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
                None => {
                    self.chrome.texture =
                        Some(ctx.load_texture("notebook", image, egui::TextureOptions::NEAREST));
                }
            }
            self.chrome.spread = Some(spread.clone());
        }
        if let Some(texture) = &self.chrome.texture {
            let size = egui::vec2(spread.size.0 as f32, spread.size.1 as f32) * layout.unit;
            ui.painter().image(
                texture.id(),
                egui::Rect::from_min_size(layout.window.min, size),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        self.cover_controls(ui, &layout);
        layout
    }

    /// The leather moves the window; the studs and tabs do what they are for.
    fn cover_controls(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        let ctx = ui.ctx().clone();
        let dark = layout.dark;
        // The leather, wherever it shows: the strip along the top, the foot, and either side.
        let leather = [
            layout.strip,
            egui::Rect::from_min_max(
                egui::pos2(layout.window.min.x, layout.status.min.y - 2.0 * layout.unit),
                layout.window.max,
            ),
            egui::Rect::from_min_max(
                layout.window.min,
                egui::pos2(layout.left.min.x, layout.window.max.y),
            ),
            egui::Rect::from_min_max(
                egui::pos2(layout.right.max.x, layout.window.min.y),
                layout.window.max,
            ),
        ];
        for (index, rect) in leather.into_iter().enumerate() {
            let response = ui.interact(
                rect,
                egui::Id::new(("leather", index)),
                egui::Sense::click_and_drag(),
            );
            if response.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if response.double_clicked() {
                let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
        }
        self.resize_edges(ui, layout);
        let mut hot = None;
        let mut down = None;
        for (glyph, rect) in &layout.studs {
            let id = stud_id(*glyph);
            let response = ui.interact(*rect, id, egui::Sense::click());
            if response.hovered() {
                hot = Some(id);
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.is_pointer_button_down_on() {
                down = Some(id);
            }
            let hint = match glyph {
                Glyph::Close => "Leave the house",
                Glyph::Shrink => "Put the notebook away for now",
                Glyph::Grow => "Open the notebook wide",
                Glyph::Camera => "Save a picture of the house (P)",
            };
            let clicked = response.clicked();
            response.on_hover_text(hint);
            if !clicked {
                continue;
            }
            match glyph {
                Glyph::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Glyph::Shrink => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                Glyph::Grow => {
                    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                Glyph::Camera => self.save_photo(),
            }
        }
        let painter = ui.painter();
        let name = painter.layout_job(label_job(
            &self.household.house_name(),
            12.0,
            ink::cover(dark),
        ));
        painter.galley(
            layout.patch.center() - name.size() / 2.0 + egui::vec2(0.0, 0.5),
            name,
            ink::cover(dark),
        );
        for (mode, rect) in layout.modes {
            let id = mode_id(mode);
            let response = ui.interact(rect, id, egui::Sense::click());
            if response.hovered() {
                hot = Some(id);
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let open = mode == self.mode;
            let colour = if open {
                ink::page(dark)
            } else {
                ink::deboss(dark)
            };
            let word = match mode {
                Mode::Live => "Live",
                Mode::Arrange => "Arrange",
            };
            let galley = ui.painter().layout_job(label_job(word, LABEL, colour));
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, colour);
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, word)
            });
            if response.clicked() {
                self.set_mode(mode);
            }
        }
        for (drawer, rect) in layout.page_tabs.clone() {
            let id = page_id(drawer);
            let response = ui.interact(rect, id, egui::Sense::click());
            if response.hovered() {
                hot = Some(id);
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let name = pages(self.mode)
                .iter()
                .find(|(which, _)| *which == drawer)
                .map_or("", |(_, name)| name);
            let open = drawer == self.page();
            let colour = if open {
                ink::forest(dark)
            } else {
                ink::muted(dark)
            };
            let galley = ui.painter().layout_job(label_job(name, LABEL, colour));
            ui.painter().galley(
                rect.center() - galley.size() / 2.0 + egui::vec2(0.0, layout.unit),
                galley,
                colour,
            );
            if response.clicked() {
                match self.mode {
                    Mode::Live => self.live_page = drawer,
                    Mode::Arrange => {
                        self.drawer = drawer;
                        self.placing = None;
                    }
                }
            }
        }
        self.chrome.hot = hot;
        self.chrome.down = down;
    }

    /// The window's edges, which resize it where the system will not do it for a window with no
    /// frame of its own. On macOS the window keeps its own edges, and needs nothing here.
    fn resize_edges(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        if cfg!(target_os = "macos") {
            return;
        }
        let ctx = ui.ctx().clone();
        let w = layout.window;
        let grip = 5.0;
        let edges = [
            (
                egui::Rect::from_min_max(w.min, egui::pos2(w.max.x, w.min.y + grip)),
                egui::ResizeDirection::North,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                egui::Rect::from_min_max(egui::pos2(w.min.x, w.max.y - grip), w.max),
                egui::ResizeDirection::South,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                egui::Rect::from_min_max(w.min, egui::pos2(w.min.x + grip, w.max.y)),
                egui::ResizeDirection::West,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                egui::Rect::from_min_max(egui::pos2(w.max.x - grip, w.min.y), w.max),
                egui::ResizeDirection::East,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                egui::Rect::from_min_max(w.max - egui::vec2(grip * 2.0, grip * 2.0), w.max),
                egui::ResizeDirection::SouthEast,
                egui::CursorIcon::ResizeNwSe,
            ),
        ];
        for (index, (rect, direction, cursor)) in edges.into_iter().enumerate() {
            let response = ui.interact(rect, egui::Id::new(("edge", index)), egui::Sense::drag());
            if response.hovered() {
                ctx.set_cursor_icon(cursor);
            }
            if response.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
    }
}

/// How the window itself is made: without the system's frame, since the notebook is its frame.
/// On macOS it keeps its shadow, rounded corners and edges to resize by, with its title bar and
/// buttons hidden; elsewhere it has no frame at all, and the cover's edges resize it.
pub fn frameless(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    if cfg!(target_os = "macos") {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(false)
    } else {
        viewport.with_decorations(false)
    }
}

/// The page's visuals: ink on paper, mint for whatever is chosen, and cards with the notebook's
/// plum outline instead of rounded grey boxes.
pub(super) fn style(ctx: &egui::Context) {
    for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
        ctx.style_mut_of(theme, |style| {
            // Names in the notes are things to pick up, not text to select: a selectable label
            // would take the click and the drag for itself.
            style.interaction.selectable_labels = false;
            let visuals = &mut style.visuals;
            visuals.override_text_color = Some(ink::page(dark));
            visuals.panel_fill = egui::Color32::TRANSPARENT;
            visuals.window_fill = ink::card(dark);
            visuals.window_stroke = egui::Stroke::new(1.0, ink::line(dark));
            visuals.window_corner_radius = egui::CornerRadius::ZERO;
            visuals.menu_corner_radius = egui::CornerRadius::ZERO;
            visuals.popup_shadow = egui::Shadow {
                offset: [2, 3],
                blur: 0,
                spread: 0,
                color: egui::Color32::from_black_alpha(60),
            };
            visuals.selection.bg_fill = ink::mint(dark);
            visuals.selection.stroke = egui::Stroke::new(1.0, ink::forest(dark));
            visuals.extreme_bg_color = ink::card(dark);
            visuals.faint_bg_color = ink::card(dark);
            let widgets = &mut visuals.widgets;
            for state in [
                &mut widgets.noninteractive,
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                state.corner_radius = egui::CornerRadius::same(1);
                state.fg_stroke = egui::Stroke::new(1.0, ink::page(dark));
            }
            widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
            widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ink::line(dark));
            widgets.inactive.bg_fill = ink::card(dark);
            widgets.inactive.weak_bg_fill = ink::card(dark);
            widgets.inactive.bg_stroke =
                egui::Stroke::new(1.0, ink::line(dark).gamma_multiply(0.6));
            widgets.hovered.bg_fill = ink::mint(dark);
            widgets.hovered.weak_bg_fill = ink::mint(dark);
            widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ink::line(dark));
            widgets.active.bg_fill = ink::mint(dark).gamma_multiply(0.85);
            widgets.active.weak_bg_fill = ink::mint(dark).gamma_multiply(0.85);
            widgets.active.bg_stroke = egui::Stroke::new(1.0, ink::forest(dark));
            style.spacing.button_padding = egui::vec2(6.0, 2.0);
            style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notebook_pixel_is_a_whole_number_of_screen_pixels() {
        for ppp in [1.0_f32, 1.25, 1.5, 2.0, 3.0] {
            let pixels = unit(ppp) * ppp;
            assert!((pixels - pixels.round()).abs() < 1e-4, "{ppp}: {pixels}");
            assert!(pixels >= 2.0);
        }
        assert_eq!(unit(2.0), 1.5);
    }
}
