//! One frame of the room: its shell, everything in it back to front, the residents among it all,
//! and whatever the owner is pointing at or carrying.
//!
//! Things are drawn in an order worked out from where they stand on the floor rather than from a
//! single depth number, so a long sofa and a resident beside it sort correctly: of two things whose
//! pictures overlap, the one wholly further back along either floor axis goes first. Whoever sits
//! on a piece is drawn with it — after its seat, before whatever of it stands in front — and
//! a tall piece that would hide someone standing behind it is drawn faded, so nobody is lost.

use crate::actor::Actor;
use crate::art::{PieceCache, Sprite, displays, shell};
use crate::catalog;
use crate::household::Id;
use crate::iso::{SCENE_HEIGHT, SCENE_WIDTH, View};
use crate::paint::{self, rgba};
use crate::room::{self, Footprint, Place, Showing};
use formiga_art::{Canvas, Rgba};
use formiga_home_contract::{DisplayId, DisplayItem, HomeSnapshot, RoomLayout, Spot};
use std::collections::HashMap;

/// How high on a wall something hangs, in pixels above the floor.
pub const HANG_HEIGHT: i32 = 32;

/// Something in the room the owner can point at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Resident(Id),
    Piece(u16),
    Shown(DisplayId),
    Floor(u8, u8),
}

/// A piece or a thing being carried in Arrange Mode, shown where it would go.
pub struct Ghost {
    pub sprite: Sprite,
    /// Where its anchor would land.
    pub at: (i32, i32),
    pub fits: bool,
    /// The tiles it would take, outlined on the floor.
    pub footprint: Option<Footprint>,
}

#[derive(Default)]
pub struct Overlay {
    pub hovered: Option<Target>,
    pub selected: Option<Id>,
    pub ghost: Option<Ghost>,
    /// Arranging: residents are drawn a little faded so the furniture reads first.
    pub arranging: bool,
    /// Something being carried is lifted out of the room while it is carried.
    pub lifted: Option<Target>,
}

/// One thing to draw, with the floor it stands on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drawn {
    Piece(usize),
    FloorThing(usize),
    Resident(usize),
}

#[derive(Clone, Copy, Debug)]
struct Placed {
    what: Drawn,
    x: (f32, f32),
    y: (f32, f32),
    /// Its picture's extent on screen, inclusive.
    rect: (i32, i32, i32, i32),
    flat: bool,
}

impl Placed {
    fn depth(&self) -> f32 {
        self.x.0 + self.x.1 + self.y.0 + self.y.1
    }

    fn overlaps_on_screen(&self, other: &Self) -> bool {
        self.rect.0 <= other.rect.2
            && other.rect.0 <= self.rect.2
            && self.rect.1 <= other.rect.3
            && other.rect.1 <= self.rect.3
    }

    /// Whether this is drawn before `other`, given that their pictures overlap. If one is wholly
    /// further back along a floor axis and the other is not along either, that one goes first.
    /// Two things apart diagonally — one further back along the width, the other along the
    /// depth — only meet on screen where their nearest corners do, so the corner further back
    /// goes first. Anything else goes by how far back each stands.
    fn before(&self, other: &Self) -> bool {
        const SLACK: f32 = 0.02;
        let back_x = self.x.1 <= other.x.0 + SLACK;
        let back_y = self.y.1 <= other.y.0 + SLACK;
        let front_x = other.x.1 <= self.x.0 + SLACK;
        let front_y = other.y.1 <= self.y.0 + SLACK;
        match (back_x || back_y, front_x || front_y) {
            (true, false) => true,
            (false, true) => false,
            _ if back_x && front_y => self.x.1 + self.y.0 < other.x.0 + other.y.1,
            _ if back_y && front_x => self.x.0 + self.y.1 < other.x.1 + other.y.0,
            _ => self.depth() < other.depth(),
        }
    }
}

/// What the shell was last drawn for: its floor, its walls, its size, and what hangs on them.
type ShellKey = (String, String, u8, u8, Vec<(DisplayId, Spot)>);

pub struct Scene {
    pub view: View,
    shell_key: Option<ShellKey>,
    shell: Canvas,
    pieces: PieceCache,
    things: HashMap<(DisplayId, PlaceKey, Showing), Sprite>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PlaceKey {
    Top,
    Shelf,
    Wall,
    Floor,
}

impl From<Place> for PlaceKey {
    fn from(place: Place) -> Self {
        match place {
            Place::Top => Self::Top,
            Place::Shelf => Self::Shelf,
            Place::Wall => Self::Wall,
            Place::Floor => Self::Floor,
        }
    }
}

impl Scene {
    pub fn new(layout: &RoomLayout) -> Self {
        Self {
            view: View::new(layout.width, layout.depth),
            shell_key: None,
            shell: Canvas::new(SCENE_WIDTH, SCENE_HEIGHT),
            pieces: PieceCache::default(),
            things: HashMap::new(),
        }
    }

    /// A thing's picture as it is shown in a place of this kind.
    pub fn thing(&mut self, item: &DisplayItem, place: Place, showing: Showing) -> &Sprite {
        self.things
            .entry((item.id.clone(), place.into(), showing))
            .or_insert_with(|| displays::sprite(item, place, showing))
    }

    pub fn piece_sprite(&mut self, piece: &'static catalog::Piece, turn: u8) -> &Sprite {
        self.pieces.get(piece, turn)
    }

    /// The shell, with whatever hangs on its walls, drawn again only when either changes.
    fn refresh_shell(&mut self, layout: &RoomLayout, snapshot: &HomeSnapshot) {
        let mut hung: Vec<_> = layout
            .displays
            .iter()
            .filter(|shown| matches!(shown.spot, Spot::Wall { .. }))
            .map(|shown| (shown.item.clone(), shown.spot))
            .collect();
        hung.sort_by_key(|shown| shown.1);
        let key = (
            layout.floor.as_str().to_owned(),
            layout.wall.as_str().to_owned(),
            layout.width,
            layout.depth,
            hung,
        );
        if self.shell_key.as_ref() == Some(&key) {
            return;
        }
        self.view = View::new(layout.width, layout.depth);
        let mut canvas = shell::draw(&self.view, layout.floor.as_str(), layout.wall.as_str());
        for (item, spot) in &key.4 {
            let Spot::Wall { side, at } = *spot else {
                continue;
            };
            let Some(item) = snapshot.item(item) else {
                continue;
            };
            let Some(showing) = room::showing(item, Place::Wall) else {
                continue;
            };
            let at_screen = self.view.on_wall(side, at, HANG_HEIGHT);
            let sprite = self.thing(item, Place::Wall, showing);
            let (ox, oy) = sprite.origin(at_screen);
            let picture = sprite.canvas.clone();
            paint::blit(&mut canvas, &picture, ox, oy);
        }
        self.shell = canvas;
        self.shell_key = Some(key);
    }

    /// Everything that stands on the floor, in the order it is drawn.
    fn order(
        &mut self,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        lifted: Option<&Target>,
    ) -> Vec<Placed> {
        let view = self.view;
        let mut placed = Vec::new();
        for (index, piece) in layout.pieces.iter().enumerate() {
            if lifted == Some(&Target::Piece(piece.uid)) {
                continue;
            }
            let footprint = room::footprint(piece);
            let (flat, rect) = match catalog::piece(&piece.piece) {
                Some(kind) => {
                    let sprite = self.pieces.get(kind, piece.turn);
                    let at = view.pixel(f32::from(piece.x), f32::from(piece.y));
                    (kind.flat, sprite_rect(sprite, at))
                }
                None => (false, tile_rect(&view, footprint)),
            };
            placed.push(Placed {
                what: Drawn::Piece(index),
                x: (f32::from(footprint.x), f32::from(footprint.x + footprint.w)),
                y: (f32::from(footprint.y), f32::from(footprint.y + footprint.d)),
                rect,
                flat,
            });
        }
        for (index, shown) in layout.displays.iter().enumerate() {
            let Spot::Floor { x, y } = shown.spot else {
                continue;
            };
            if lifted == Some(&Target::Shown(shown.item.clone())) {
                continue;
            }
            let Some(item) = snapshot.item(&shown.item) else {
                continue;
            };
            let at = floor_anchor(&view, x, y);
            let sprite = self.thing(item, Place::Floor, Showing::Itself);
            placed.push(Placed {
                what: Drawn::FloorThing(index),
                x: (f32::from(x) + 0.15, f32::from(x) + 0.85),
                y: (f32::from(y) + 0.15, f32::from(y) + 0.85),
                rect: sprite_rect(sprite, at),
                flat: false,
            });
        }
        for (index, actor) in actors.iter_mut().enumerate() {
            if actor.on_piece.is_some() || lifted == Some(&Target::Resident(actor.id)) {
                continue;
            }
            let (x, y) = actor.pos;
            let (l, t, r, b) = actor.bounds(&view, now);
            placed.push(Placed {
                what: Drawn::Resident(index),
                x: (x - 0.22, x + 0.22),
                y: (y - 0.22, y + 0.22),
                rect: (l, t, r, b + 3),
                flat: false,
            });
        }
        sort_back_to_front(placed)
    }

    /// The room as it is at `now`.
    pub fn compose(
        &mut self,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        overlay: &Overlay,
    ) -> Canvas {
        self.refresh_shell(layout, snapshot);
        let mut canvas = self.shell.clone();
        let view = self.view;
        let order = self.order(layout, snapshot, actors, now, overlay.lifted.as_ref());
        let resident_opacity = if overlay.arranging { 170 } else { 255 };
        // Who has been drawn so far, for the tall pieces that would hide them.
        let mut drawn_residents: Vec<(i32, i32, i32, i32)> = Vec::new();
        for entry in &order {
            match entry.what {
                Drawn::Piece(index) => {
                    let piece = &layout.pieces[index];
                    let hides = !entry.flat
                        && catalog::piece(&piece.piece).is_some_and(|kind| kind.tall())
                        && drawn_residents
                            .iter()
                            .any(|rect| rects_meet(*rect, entry.rect));
                    self.draw_piece(
                        &mut canvas,
                        layout,
                        snapshot,
                        actors,
                        index,
                        now,
                        hides,
                        resident_opacity,
                    );
                }
                Drawn::FloorThing(index) => {
                    let shown = &layout.displays[index];
                    let (Spot::Floor { x, y }, Some(item)) =
                        (shown.spot, snapshot.item(&shown.item))
                    else {
                        continue;
                    };
                    let at = floor_anchor(&view, x, y);
                    let sprite = self.thing(item, Place::Floor, Showing::Itself);
                    let (ox, oy) = sprite.origin(at);
                    let picture = sprite.canvas.clone();
                    paint::blit(&mut canvas, &picture, ox, oy);
                }
                Drawn::Resident(index) => {
                    let actor = &mut actors[index];
                    actor.draw_shadow(&mut canvas, &view);
                    if overlay.selected == Some(actor.id) {
                        selection_ring(&mut canvas, &view, actor.pos);
                    }
                    actor.draw(&mut canvas, &view, now, resident_opacity);
                    drawn_residents.push(entry.rect);
                }
            }
        }
        if !overlay.arranging {
            for actor in actors.iter_mut() {
                actor.draw_cue(&mut canvas, &view, now);
            }
        }
        if let Some(target) = &overlay.hovered {
            self.ring(&mut canvas, layout, snapshot, actors, now, target);
        }
        if let Some(ghost) = &overlay.ghost {
            let tint = if ghost.fits {
                rgba(0x7fd08a, 255)
            } else {
                rgba(0xe0606a, 255)
            };
            if let Some(footprint) = ghost.footprint {
                outline_tiles(&mut canvas, &view, footprint, paint::faded(tint, 200));
            }
            let (ox, oy) = ghost.sprite.origin(ghost.at);
            paint::blit_tinted(&mut canvas, &ghost.sprite.canvas, ox, oy, tint, 210);
            if let Some(over) = &ghost.sprite.over {
                paint::blit_tinted(&mut canvas, over, ox, oy, tint, 210);
            }
        }
        canvas
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_piece(
        &mut self,
        canvas: &mut Canvas,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        index: usize,
        now: f32,
        faded: bool,
        resident_opacity: u8,
    ) {
        let view = self.view;
        let placed = &layout.pieces[index];
        let at = view.pixel(f32::from(placed.x), f32::from(placed.y));
        let Some(kind) = catalog::piece(&placed.piece) else {
            let sprite = crate::art::furniture::draw(&UNKNOWN, false);
            let (ox, oy) = sprite.origin(at);
            paint::blit(canvas, &sprite.canvas, ox, oy);
            return;
        };
        let opacity = if faded { 120 } else { 255 };
        let sprite = self.pieces.get(kind, placed.turn).clone();
        let (ox, oy) = sprite.origin(at);
        let mut sitters: Vec<usize> = actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| actor.on_piece == Some(placed.uid))
            .map(|(index, _)| index)
            .collect();
        sitters.sort_by(|a, b| {
            let depth = |actor: &Actor| actor.pos.0 + actor.pos.1;
            depth(&actors[*a]).total_cmp(&depth(&actors[*b]))
        });
        paint::blit_faded(canvas, &sprite.canvas, ox, oy, opacity);
        // What is shown on it, lowest first.
        let mut shown: Vec<_> = layout
            .displays
            .iter()
            .filter_map(|shown| match shown.spot {
                Spot::On { piece, slot } if piece == placed.uid => Some((slot, &shown.item)),
                _ => None,
            })
            .collect();
        shown.sort_by_key(|(slot, _)| *slot);
        let surfaces = room::surfaces(placed);
        for (slot, item) in shown {
            let (Some(surface), Some(item)) =
                (surfaces.get(usize::from(slot)), snapshot.item(item))
            else {
                continue;
            };
            let place = match surface.holds {
                catalog::Holds::Top => Place::Top,
                catalog::Holds::Shelf => Place::Shelf,
            };
            let Some(showing) = room::showing(item, place) else {
                continue;
            };
            let (sx, sy) = view.screen(surface.at.0, surface.at.1);
            let point = (sx.round() as i32, sy.round() as i32 - surface.height);
            let sprite = self.thing(item, place, showing);
            let (ix, iy) = sprite.origin(point);
            let picture = sprite.canvas.clone();
            paint::blit(canvas, &picture, ix, iy);
        }
        for &sitter in &sitters {
            actors[sitter].draw(canvas, &view, now, resident_opacity);
        }
        if let Some(over) = &sprite.over {
            paint::blit_faded(canvas, over, ox, oy, opacity);
        }
    }

    /// A ring round whatever the owner is pointing at.
    fn ring(
        &mut self,
        canvas: &mut Canvas,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        target: &Target,
    ) {
        let view = self.view;
        let color = rgba(0xfff3c4, 230);
        match target {
            Target::Resident(id) => {
                if let Some(actor) = actors.iter_mut().find(|actor| actor.id == *id) {
                    let (l, t, r, b) = actor.bounds(&view, now);
                    let mut frame = Canvas::new((r - l + 1) as u32, (b - t + 1) as u32);
                    let snap = canvas.clone();
                    for y in 0..frame.height() as i32 {
                        for x in 0..frame.width() as i32 {
                            if actor.covers(&view, now, (l + x, t + y)) {
                                frame.set(x, y, snap.get(l + x, t + y));
                            }
                        }
                    }
                    paint::ring(canvas, &frame, l, t, color);
                }
            }
            Target::Piece(uid) => {
                if let Some(placed) = layout.piece(*uid)
                    && let Some(kind) = catalog::piece(&placed.piece)
                {
                    let sprite = self.pieces.get(kind, placed.turn);
                    let at = view.pixel(f32::from(placed.x), f32::from(placed.y));
                    let (ox, oy) = sprite.origin(at);
                    let picture = sprite.canvas.clone();
                    paint::ring(canvas, &picture, ox, oy, color);
                }
            }
            Target::Shown(item) => {
                if let Some((sprite, at)) = self.shown_sprite(layout, snapshot, item) {
                    let (ox, oy) = sprite.origin(at);
                    paint::ring(canvas, &sprite.canvas, ox, oy, color);
                }
            }
            Target::Floor(x, y) => {
                let footprint = Footprint {
                    x: *x,
                    y: *y,
                    w: 1,
                    d: 1,
                };
                outline_tiles(canvas, &view, footprint, paint::faded(color, 170));
            }
        }
    }

    /// Where something shown at `spot` has its anchor on the scene.
    pub fn spot_anchor(&self, layout: &RoomLayout, spot: Spot) -> Option<(i32, i32)> {
        let view = self.view;
        Some(match spot {
            Spot::Wall { side, at } => view.on_wall(side, at, HANG_HEIGHT),
            Spot::Floor { x, y } => floor_anchor(&view, x, y),
            Spot::On { piece, slot } => {
                let surface = room::surfaces(layout.piece(piece)?)
                    .into_iter()
                    .nth(usize::from(slot))?;
                let (sx, sy) = view.screen(surface.at.0, surface.at.1);
                (sx.round() as i32, sy.round() as i32 - surface.height)
            }
        })
    }

    /// A shown thing's picture and where its anchor is, wherever it is shown.
    fn shown_sprite(
        &mut self,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        item: &DisplayId,
    ) -> Option<(Sprite, (i32, i32))> {
        let shown = layout.displays.iter().find(|shown| &shown.item == item)?;
        let thing = snapshot.item(item)?;
        let place = room::place_of(layout, shown.spot)?;
        let showing = room::showing(thing, place)?;
        let at = self.spot_anchor(layout, shown.spot)?;
        Some((self.thing(thing, place, showing).clone(), at))
    }

    /// What is under a point of the scene: a resident first, then what is shown, then furniture,
    /// then the floor.
    pub fn hit(
        &mut self,
        layout: &RoomLayout,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        point: (i32, i32),
    ) -> Option<Target> {
        self.refresh_shell(layout, snapshot);
        let view = self.view;
        for actor in actors.iter_mut() {
            if actor.covers(&view, now, point) {
                return Some(Target::Resident(actor.id));
            }
        }
        for shown in &layout.displays {
            if let Some((sprite, at)) = self.shown_sprite(layout, snapshot, &shown.item)
                && sprite.covers(at, point)
            {
                return Some(Target::Shown(shown.item.clone()));
            }
        }
        let order = self.order(layout, snapshot, actors, now, None);
        for entry in order.iter().rev() {
            if let Drawn::Piece(index) = entry.what {
                let placed = &layout.pieces[index];
                let Some(kind) = catalog::piece(&placed.piece) else {
                    continue;
                };
                let sprite = self.pieces.get(kind, placed.turn);
                let at = view.pixel(f32::from(placed.x), f32::from(placed.y));
                if sprite.covers(at, point) && !kind.flat {
                    return Some(Target::Piece(placed.uid));
                }
            }
        }
        let tile = view.tile_at(point.0 as f32 + 0.5, point.1 as f32 + 0.5)?;
        // A rug is picked by its floor, once nothing standing on it was.
        let rug = layout.pieces.iter().find(|placed| {
            catalog::piece(&placed.piece).is_some_and(|kind| kind.flat)
                && room::footprint(placed).contains(tile.0, tile.1)
        });
        Some(match rug {
            Some(rug) => Target::Piece(rug.uid),
            None => Target::Floor(tile.0, tile.1),
        })
    }
}

/// A piece this build does not know, drawn as a plain crate.
static UNKNOWN: catalog::Piece = catalog::Piece {
    id: "unknown",
    name: "Something",
    family: catalog::Family::Toys,
    size: (1, 1),
    height: 10,
    flat: false,
    uses: &[],
    surfaces: &[],
    arrives: catalog::Arrival::Always,
    lift: 0,
};

/// Where a thing standing on a floor tile has its foot.
pub fn floor_anchor(view: &View, x: u8, y: u8) -> (i32, i32) {
    let (sx, sy) = view.tile_centre(x, y);
    (sx.round() as i32, sy.round() as i32 + 1)
}

fn sprite_rect(sprite: &Sprite, at: (i32, i32)) -> (i32, i32, i32, i32) {
    let (ox, oy) = sprite.origin(at);
    match sprite.canvas.alpha_bounds() {
        Some((l, t, r, b)) => (ox + l as i32, oy + t as i32, ox + r as i32, oy + b as i32),
        None => (ox, oy, ox, oy),
    }
}

fn tile_rect(view: &View, footprint: Footprint) -> (i32, i32, i32, i32) {
    let corners = view.footprint(footprint.x, footprint.y, footprint.w, footprint.d);
    let xs = corners.iter().map(|c| c.0);
    let ys = corners.iter().map(|c| c.1);
    (
        xs.clone().fold(f32::MAX, f32::min) as i32,
        ys.clone().fold(f32::MAX, f32::min) as i32 - 12,
        xs.fold(f32::MIN, f32::max) as i32,
        ys.fold(f32::MIN, f32::max) as i32,
    )
}

fn rects_meet(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

/// Rugs first; then everything else so that of any two whose pictures overlap, the one wholly
/// further back goes first, ties and tangles settled by how far back each stands.
fn sort_back_to_front(placed: Vec<Placed>) -> Vec<Placed> {
    let (mut flat, standing): (Vec<_>, Vec<_>) = placed.into_iter().partition(|p| p.flat);
    flat.sort_by(|a, b| a.depth().total_cmp(&b.depth()));
    let count = standing.len();
    let mut before = vec![Vec::new(); count];
    let mut waiting = vec![0_usize; count];
    for a in 0..count {
        for b in 0..count {
            if a == b || !standing[a].overlaps_on_screen(&standing[b]) {
                continue;
            }
            let (ab, ba) = (
                standing[a].before(&standing[b]),
                standing[b].before(&standing[a]),
            );
            // Each pair decides once, and a tie goes to the one listed first.
            let a_first = ab && (!ba || a < b);
            if a_first {
                before[a].push(b);
                waiting[b] += 1;
            }
        }
    }
    let mut done = vec![false; count];
    let mut out = flat;
    for _ in 0..count {
        // The furthest back of those with nothing left to wait for; in a tangle, the furthest
        // back of all.
        let next = (0..count)
            .filter(|&i| !done[i] && waiting[i] == 0)
            .min_by(|&a, &b| standing[a].depth().total_cmp(&standing[b].depth()))
            .or_else(|| {
                (0..count)
                    .filter(|&i| !done[i])
                    .min_by(|&a, &b| standing[a].depth().total_cmp(&standing[b].depth()))
            });
        let Some(next) = next else { break };
        done[next] = true;
        for &after in &before[next] {
            waiting[after] = waiting[after].saturating_sub(1);
        }
        out.push(standing[next]);
    }
    out
}

/// A soft gold ring on the floor round the chosen resident's feet.
fn selection_ring(canvas: &mut Canvas, view: &View, (x, y): (f32, f32)) {
    let color = rgba(0xf4c95d, 220);
    let r = 0.42;
    let points = [
        view.pixel(x - r, y - r),
        view.pixel(x + r, y - r),
        view.pixel(x + r, y + r),
        view.pixel(x - r, y + r),
    ];
    for index in 0..4 {
        paint::line(canvas, points[index], points[(index + 1) % 4], color);
    }
}

/// The edges of a block of tiles, drawn on the floor.
fn outline_tiles(canvas: &mut Canvas, view: &View, footprint: Footprint, color: Rgba) {
    let corners = view.footprint(footprint.x, footprint.y, footprint.w, footprint.d);
    let px = |c: (f32, f32)| (c.0.round() as i32, c.1.round() as i32);
    for index in 0..4 {
        paint::line(
            canvas,
            px(corners[index]),
            px(corners[(index + 1) % 4]),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: (f32, f32), y: (f32, f32), what: Drawn) -> Placed {
        Placed {
            what,
            x,
            y,
            rect: (0, 0, 100, 100),
            flat: false,
        }
    }

    #[test]
    fn of_two_things_that_overlap_on_screen_the_one_further_back_goes_first() {
        let sofa = at((2.0, 4.0), (3.0, 4.0), Drawn::Piece(0));
        // Standing just in front of the sofa's far end, but further back by a plain depth sum
        // than the sofa's middle.
        let beside = at((1.6, 2.0), (4.2, 4.6), Drawn::Resident(0));
        let behind = at((2.5, 2.9), (2.2, 2.6), Drawn::Resident(1));
        let order = sort_back_to_front(vec![beside, sofa, behind]);
        let names: Vec<_> = order.iter().map(|p| p.what).collect();
        assert_eq!(
            names,
            vec![Drawn::Resident(1), Drawn::Piece(0), Drawn::Resident(0)]
        );
    }

    #[test]
    fn rugs_are_always_under_everything() {
        let mut rug = at((0.0, 4.0), (0.0, 4.0), Drawn::Piece(1));
        rug.flat = true;
        let chair = at((0.0, 1.0), (0.0, 1.0), Drawn::Piece(0));
        let order = sort_back_to_front(vec![chair, rug]);
        assert_eq!(order[0].what, Drawn::Piece(1));
    }
}
