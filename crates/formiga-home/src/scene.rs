//! One frame of the house: its shell, everything in it back to front, the residents among it all,
//! and whatever the owner is pointing at or carrying.
//!
//! Things are drawn in an order worked out from where they stand on the floor rather than from a
//! single depth number, so a long sofa and a resident beside it sort correctly: of two things whose
//! pictures overlap, the one wholly further back along either floor axis goes first. Whoever sits
//! on a piece is drawn with it — after its seat, before whatever of it stands in front.
//!
//! In a cutaway, a piece can stand in front of what it should not hide: a tall piece in front of
//! someone, or a piece by a wall cut down low in front of what is in the room behind. Such a piece
//! is drawn see-through, but only where it covers them and in their own shape, so nobody and
//! nothing is lost and the piece is otherwise drawn as it is.

use crate::actor::Actor;
use crate::art::{PieceCache, Sprite, displays, shell};
use crate::catalog;
use crate::house::{At, Height, House, Room, Wall};
use crate::household::Id;
use crate::iso::View;
use crate::paint::{self, rgba};
use crate::room::{self, Footprint, Place, Showing};
use formiga_art::{Canvas, Rgba};
use formiga_home_contract::{DisplayId, DisplayItem, HomeSnapshot};
use std::collections::HashMap;

/// How high on a wall something hangs, in pixels above the floor.
pub const HANG_HEIGHT: i32 = 32;

/// Something in the house the owner can point at. A piece goes by its name in the house, and the
/// floor by a tile of the house's floor.
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
    /// Lamps switched off.
    pub lamps_off: Vec<u16>,
    /// The table-top light behind the house, as a photo has it; otherwise the picture is clear
    /// round the house, for the page it is drawn on.
    pub backdrop: bool,
    /// A room still being placed, picked out so it reads as the new one.
    pub new_room: Option<u8>,
    /// The room the owner is choosing finishes for, outlined.
    pub chosen_room: Option<u8>,
    /// A doorway being carried: the stretch of wall it would go in, and whether it fits there.
    pub door: Option<(Wall, bool)>,
}

/// What has been drawn so far that a piece standing in front of it could hide, pixel by pixel:
/// anyone at all, and whatever stands in each room.
struct Behind {
    width: i32,
    height: i32,
    marks: Vec<u8>,
}

/// Nothing behind a pixel; someone; or, from `THING` up, something standing in a room.
const NOBODY: u8 = 0;
const SOMEONE: u8 = 1;
const THING: u8 = 2;

impl Behind {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width: width as i32,
            height: height as i32,
            marks: vec![NOBODY; width as usize * height as usize],
        }
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return NOBODY;
        }
        self.marks[(y * self.width + x) as usize]
    }

    fn mark(&mut self, x: i32, y: i32, mark: u8) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.marks[(y * self.width + x) as usize] = mark;
        }
    }

    /// Wherever a picture drawn at `(x, y)` covers the scene.
    fn mark_picture(&mut self, picture: &Canvas, x: i32, y: i32, mark: u8) {
        for py in 0..picture.height() as i32 {
            for px in 0..picture.width() as i32 {
                if picture.get(px, py).a > 40 {
                    self.mark(x + px, y + py, mark);
                }
            }
        }
    }

    /// Wherever someone covers the scene.
    fn mark_actor(&mut self, actor: &mut Actor, view: &View, now: f32) {
        let (l, t, r, b) = actor.bounds(view, now);
        for y in t..=b {
            for x in l..=r {
                if actor.covers(view, now, (x, y)) {
                    self.mark(x, y, SOMEONE);
                }
            }
        }
    }
}

/// One thing to draw, with the floor it stands on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drawn {
    Piece(usize),
    FloorThing(usize),
    Resident(usize),
    /// A wall cut down low, by its place among the house's walls.
    Wall(usize),
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

/// What the shell was last drawn for: the rooms, their walls, what hangs on them, and whether
/// with the backdrop.
type ShellKey = (Vec<Room>, Vec<Wall>, Vec<(DisplayId, At)>, bool);

pub struct Scene {
    pub view: View,
    shell_key: Option<ShellKey>,
    shell: Canvas,
    pieces: PieceCache,
    things: HashMap<(DisplayId, PlaceKey, Showing), Sprite>,
    /// The household's keepsakes' pictures, by id: whoever is in each, in their own colours.
    pictures: HashMap<DisplayId, Canvas>,
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
    pub fn new(house: &House) -> Self {
        let view = View::of(house);
        Self {
            view,
            shell_key: None,
            shell: Canvas::new(view.size.0, view.size.1),
            pieces: PieceCache::default(),
            things: HashMap::new(),
            pictures: HashMap::new(),
        }
    }

    /// The household's keepsakes as they now are. A keepsake let go can leave its number to the
    /// next, so whatever was drawn of any keepsake is drawn again.
    pub fn set_pictures(&mut self, pictures: HashMap<DisplayId, Canvas>) {
        self.things
            .retain(|(id, _, _), _| id.source() != DisplayId::MEMENTO);
        self.shell_key = None;
        self.pictures = pictures;
    }

    /// A thing's own picture: a keepsake's with whoever is in it.
    pub fn icon(&self, item: &DisplayItem) -> Canvas {
        self.pictures
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| displays::icon(item))
    }

    /// A thing's picture as it is shown in a place of this kind.
    pub fn thing(&mut self, item: &DisplayItem, place: Place, showing: Showing) -> &Sprite {
        let key = (item.id.clone(), place.into(), showing);
        if !self.things.contains_key(&key) {
            let sprite = displays::dress(self.icon(item), item, place, showing);
            self.things.insert(key.clone(), sprite);
        }
        &self.things[&key]
    }

    pub fn piece_sprite(&mut self, piece: &'static catalog::Piece, turn: u8) -> &Sprite {
        self.pieces.get(piece, turn)
    }

    /// The shell, with whatever hangs on its walls, drawn again only when either changes. The
    /// house's picture is as big as the house needs.
    pub fn refresh_shell(&mut self, house: &House, snapshot: &HomeSnapshot, backdrop: bool) {
        let mut hung: Vec<_> = house
            .shown
            .iter()
            .filter(|shown| matches!(shown.at, At::Wall { .. }))
            .map(|shown| (shown.item.clone(), shown.at))
            .collect();
        hung.sort_by_key(|shown| shown.1);
        let key = (house.rooms.clone(), house.walls.clone(), hung, backdrop);
        if self.shell_key.as_ref() == Some(&key) {
            return;
        }
        self.view = View::of(house);
        let mut canvas = shell::draw(&self.view, house, backdrop);
        for (item, at) in &key.2 {
            let At::Wall { room, side, at } = *at else {
                continue;
            };
            let Some(wall) = house
                .wall(room, side, at)
                .filter(|wall| wall.height == Height::Full && !wall.door)
            else {
                continue;
            };
            let Some(item) = snapshot.item(item) else {
                continue;
            };
            let Some(showing) = room::showing(item, Place::Wall) else {
                continue;
            };
            let at_screen = self.view.on_wall(wall, HANG_HEIGHT);
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
        house: &House,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        lifted: Option<&Target>,
    ) -> Vec<Placed> {
        let view = self.view;
        let mut placed = Vec::new();
        for (index, piece) in house.pieces.iter().enumerate() {
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
        for (index, shown) in house.shown.iter().enumerate() {
            let At::Floor { x, y } = shown.at else {
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
            if actor.hidden
                || actor.on_piece.is_some()
                || lifted == Some(&Target::Resident(actor.id))
            {
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
        // A wall cut down low stands on the line between two tiles: in front of the one behind
        // it, and behind its own.
        for (index, wall) in house.walls.iter().enumerate() {
            if wall.height != Height::Low || wall.door {
                continue;
            }
            let (a, b) = wall.foot();
            placed.push(Placed {
                what: Drawn::Wall(index),
                x: (a.0, b.0),
                y: (a.1, b.1),
                rect: shell::low_wall_rect(&view, wall),
                flat: false,
            });
        }
        sort_back_to_front(placed)
    }

    /// The house as it is at `now`, in a picture as big as [`Self::view`] says.
    pub fn compose(
        &mut self,
        house: &House,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        overlay: &Overlay,
    ) -> Canvas {
        self.refresh_shell(house, snapshot, overlay.backdrop);
        let mut canvas = self.shell.clone();
        let view = self.view;
        // The front door stands open while anyone is in its doorway.
        for wall in house
            .walls
            .iter()
            .filter(|wall| wall.opens_outside() && wall.height == Height::Full)
        {
            let (mx, my) = wall.middle();
            let passing = actors.iter().any(|actor| {
                !actor.hidden && (actor.pos.0 - mx).powi(2) + (actor.pos.1 - my).powi(2) < 0.7
            });
            if passing {
                shell::front_door(&mut canvas, &view, wall, true);
            }
        }
        // A light that is on, and how high its bulb glows.
        let lit = |placed: &formiga_home_contract::PlacedPiece| {
            catalog::piece(&placed.piece)
                .and_then(|piece| piece.glow())
                .filter(|_| !overlay.lamps_off.contains(&placed.uid))
        };
        for placed in house.pieces.iter().filter(|placed| lit(placed).is_some()) {
            let (cx, cy) = room::footprint(placed).centre();
            crate::art::furniture::lamp_pool(&mut canvas, view.pixel(cx, cy));
        }
        let order = self.order(house, snapshot, actors, now, overlay.lifted.as_ref());
        let resident_opacity = if overlay.arranging { 170 } else { 255 };
        // What has been drawn that a piece in front of it could hide.
        let mut behind = Behind::new(canvas.width(), canvas.height());
        for entry in &order {
            match entry.what {
                Drawn::Piece(index) => {
                    let piece = &house.pieces[index];
                    self.draw_piece(
                        &mut canvas,
                        house,
                        snapshot,
                        actors,
                        index,
                        now,
                        &mut behind,
                        resident_opacity,
                    );
                    if let Some(glow) = lit(piece) {
                        let (cx, cy) = room::footprint(piece).centre();
                        crate::art::furniture::lamp_bulb(&mut canvas, view.pixel(cx, cy), glow);
                    }
                }
                Drawn::FloorThing(index) => {
                    let shown = &house.shown[index];
                    let (At::Floor { x, y }, Some(item)) = (shown.at, snapshot.item(&shown.item))
                    else {
                        continue;
                    };
                    let at = floor_anchor(&view, x, y);
                    let sprite = self.thing(item, Place::Floor, Showing::Itself);
                    let (ox, oy) = sprite.origin(at);
                    let picture = sprite.canvas.clone();
                    paint::blit(&mut canvas, &picture, ox, oy);
                    let room = house.room_at(i32::from(x), i32::from(y)).unwrap_or(0);
                    behind.mark_picture(&picture, ox, oy, THING + room);
                }
                Drawn::Resident(index) => {
                    let actor = &mut actors[index];
                    actor.draw_shadow(&mut canvas, &view);
                    if overlay.selected == Some(actor.id) {
                        selection_ring(&mut canvas, &view, actor.pos);
                    }
                    actor.draw(&mut canvas, &view, now, resident_opacity);
                    behind.mark_actor(actor, &view, now);
                }
                Drawn::Wall(index) => {
                    shell::low_wall(&mut canvas, &view, house, &house.walls[index]);
                }
            }
        }
        if !overlay.arranging {
            for actor in actors.iter_mut().filter(|actor| !actor.hidden) {
                actor.draw_cue(&mut canvas, &view, now);
            }
        }
        if let Some(target) = &overlay.hovered {
            self.ring(&mut canvas, house, snapshot, actors, now, target);
        }
        if let Some(footprint) = overlay.new_room.and_then(|room| house.footprint(room)) {
            outline_tiles(&mut canvas, &view, footprint, rgba(0x7fd08a, 230));
        }
        if let Some(footprint) = overlay.chosen_room.and_then(|room| house.footprint(room)) {
            outline_tiles(&mut canvas, &view, footprint, rgba(0xf4c95d, 230));
        }
        if let Some((wall, fits)) = &overlay.door {
            let tint = if *fits {
                rgba(0x7fd08a, 230)
            } else {
                rgba(0xe0606a, 230)
            };
            let rise = match wall.height {
                Height::Full => 40,
                Height::Low => shell::LOW_WALL,
            };
            let (a, b) = wall.foot();
            let (a, b) = (view.pixel(a.0, a.1), view.pixel(b.0, b.1));
            for (from, to) in [
                (a, b),
                ((b.0, b.1 - rise), (a.0, a.1 - rise)),
                (a, (a.0, a.1 - rise)),
                (b, (b.0, b.1 - rise)),
            ] {
                paint::line(&mut canvas, from, to, tint);
            }
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
        house: &House,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        index: usize,
        now: f32,
        behind: &mut Behind,
        resident_opacity: u8,
    ) {
        let view = self.view;
        let placed = &house.pieces[index];
        let at = view.pixel(f32::from(placed.x), f32::from(placed.y));
        let room = house
            .room_at(i32::from(placed.x), i32::from(placed.y))
            .unwrap_or(0);
        let Some(kind) = catalog::piece(&placed.piece) else {
            let sprite = crate::art::furniture::draw(&UNKNOWN, false);
            let (ox, oy) = sprite.origin(at);
            paint::blit(canvas, &sprite.canvas, ox, oy);
            behind.mark_picture(&sprite.canvas, ox, oy, THING + room);
            return;
        };
        // See-through where it would hide someone, if it is tall enough to, or where it stands in
        // front of what is in another room.
        let (tall, stands) = (kind.tall(), !kind.flat && kind.height >= 16);
        let hidden = &*behind;
        let see_through = |x: i32, y: i32| match hidden.at(x, y) {
            SOMEONE if tall => 110,
            mark if mark >= THING && stands && mark - THING != room => 110,
            _ => 255,
        };
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
        paint::blit_through(canvas, &sprite.canvas, ox, oy, see_through);
        // What is shown on it, lowest first.
        let mut shown: Vec<_> = house
            .shown
            .iter()
            .filter_map(|shown| match shown.at {
                At::On { piece, slot } if piece == placed.uid => Some((slot, &shown.item)),
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
            let point = surface_anchor(&view, surface);
            let thing = self.thing(item, place, showing);
            let (ix, iy) = thing.origin(point);
            let picture = thing.canvas.clone();
            // Anything taller than its shelf's room, shown there before shelves were measured,
            // is trimmed at what is over it rather than drawn through it.
            let top = surface
                .clearance()
                .map_or(i32::MIN, |clearance| point.1 - clearance);
            paint::blit_through(
                canvas,
                &picture,
                ix,
                iy,
                |_, y| if y < top { 0 } else { 255 },
            );
            if let Some(Some(lid)) = sprite.lids.get(usize::from(slot)) {
                paint::blit_through(canvas, lid, ox, oy, see_through);
            }
        }
        if let Some(over) = &sprite.over {
            for &sitter in &sitters {
                actors[sitter].draw(canvas, &view, now, resident_opacity);
            }
            paint::blit_through(canvas, over, ox, oy, see_through);
        } else {
            for &sitter in &sitters {
                actors[sitter].draw(canvas, &view, now, resident_opacity);
            }
        }
        // Now it, and whoever is on it, may be hidden by what stands in front.
        if !kind.flat {
            behind.mark_picture(&sprite.canvas, ox, oy, THING + room);
            if let Some(over) = &sprite.over {
                behind.mark_picture(over, ox, oy, THING + room);
            }
        }
        for &sitter in &sitters {
            behind.mark_actor(&mut actors[sitter], &view, now);
        }
    }

    /// A ring round whatever the owner is pointing at.
    fn ring(
        &mut self,
        canvas: &mut Canvas,
        house: &House,
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
                if let Some(placed) = house.piece(*uid)
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
                if let Some((sprite, at)) = self.shown_sprite(house, snapshot, item) {
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

    /// Where something shown at `at` has its anchor on the scene.
    pub fn spot_anchor(&self, house: &House, at: At) -> Option<(i32, i32)> {
        let view = self.view;
        Some(match at {
            At::Wall { room, side, at } => view.on_wall(house.wall(room, side, at)?, HANG_HEIGHT),
            At::Floor { x, y } => floor_anchor(&view, x, y),
            At::On { piece, slot } => {
                let surface = room::surfaces(house.piece(piece)?)
                    .into_iter()
                    .nth(usize::from(slot))?;
                surface_anchor(&view, &surface)
            }
        })
    }

    /// A shown thing's picture and where its anchor is, wherever it is shown.
    fn shown_sprite(
        &mut self,
        house: &House,
        snapshot: &HomeSnapshot,
        item: &DisplayId,
    ) -> Option<(Sprite, (i32, i32))> {
        let shown = house.shown.iter().find(|shown| &shown.item == item)?;
        let thing = snapshot.item(item)?;
        let place = house.place_of(shown.at)?;
        let showing = room::showing(thing, place)?;
        let at = self.spot_anchor(house, shown.at)?;
        Some((self.thing(thing, place, showing).clone(), at))
    }

    /// What is under a point of the scene: a resident first, then what is shown, then furniture,
    /// then the floor.
    pub fn hit(
        &mut self,
        house: &House,
        snapshot: &HomeSnapshot,
        actors: &mut [Actor],
        now: f32,
        point: (i32, i32),
    ) -> Option<Target> {
        self.view = View::of(house);
        let view = self.view;
        for actor in actors.iter_mut().filter(|actor| !actor.hidden) {
            if actor.covers(&view, now, point) {
                return Some(Target::Resident(actor.id));
            }
        }
        for shown in &house.shown {
            if let Some((sprite, at)) = self.shown_sprite(house, snapshot, &shown.item)
                && sprite.covers(at, point)
            {
                return Some(Target::Shown(shown.item.clone()));
            }
        }
        let order = self.order(house, snapshot, actors, now, None);
        for entry in order.iter().rev() {
            if let Drawn::Piece(index) = entry.what {
                let placed = &house.pieces[index];
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
        house.room_at(i32::from(tile.0), i32::from(tile.1))?;
        // A rug is picked by its floor, once nothing standing on it was.
        let rug = house.pieces.iter().find(|placed| {
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
    set: catalog::Set::Home,
};

/// How far in front of a shelf's middle, in pixels down the picture, a thing stands when there is
/// a board over it: at the board's front, where the board hides as little of it as it can. In the
/// picture rather than on the floor, so it is the same at every turn.
const SHELF_FRONT: i32 = 3;

/// Where a thing shown on a surface has its anchor on the scene.
fn surface_anchor(view: &View, surface: &room::SurfaceAt) -> (i32, i32) {
    let (sx, sy) = view.screen(surface.at.0, surface.at.1);
    let front = match surface.cover {
        Some(catalog::Cover::Board(_)) => SHELF_FRONT,
        _ => 0,
    };
    (
        sx.round() as i32,
        sy.round() as i32 - surface.height + front,
    )
}

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
