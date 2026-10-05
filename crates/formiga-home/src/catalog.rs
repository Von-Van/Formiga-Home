//! Everything Home can put in a room, by the identifier a layout keeps.
//!
//! Nothing here costs anything. The starter pieces are there from the first visit; a few more
//! arrive as the colony lives, or as it finds things worth a case, and once they have arrived they
//! stay. An identifier is never reused for something else, so a layout always means what it meant
//! when it was arranged; a piece a newer Home has, and this one does not, is kept in its place and
//! left alone.

use formiga_home_contract::{CatalogId, HouseStyle};

/// What a piece of furniture is for, as a companion sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Use {
    /// Somewhere to sit: `seats` of them, side by side along the piece's width.
    Sit { seats: u8 },
    /// Somewhere to curl up for a nap.
    Nap,
    /// Somewhere to properly sleep.
    Sleep,
    /// Something to play with.
    Play,
    /// Something to eat from.
    Snack,
}

/// What a surface takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Holds {
    /// A table top: small things, as themselves, or anything on a card.
    Top,
    /// A shelf or a case: small or precious things, as themselves, or anything on a card.
    Shelf,
}

/// One place on a piece where something can be shown: a point on its footprint, in tiles from the
/// footprint's far corner as the piece stands unturned, and how high above the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub at: (f32, f32),
    pub height: i32,
    pub holds: Holds,
}

/// When a piece is in the catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrival {
    Always,
    /// Once the colony has lived this many days.
    AfterDays(u32),
    /// Once the colony has this many things to show.
    AfterFinds(usize),
}

/// How the catalogue groups what it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    Seats,
    Beds,
    Tables,
    Shelves,
    Rugs,
    Lights,
    Plants,
    Toys,
    Food,
}

impl Family {
    pub const ALL: [Self; 9] = [
        Self::Seats,
        Self::Beds,
        Self::Tables,
        Self::Shelves,
        Self::Rugs,
        Self::Lights,
        Self::Plants,
        Self::Toys,
        Self::Food,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Seats => "Seats",
            Self::Beds => "Beds",
            Self::Tables => "Tables",
            Self::Shelves => "Shelves and cases",
            Self::Rugs => "Rugs",
            Self::Lights => "Lights",
            Self::Plants => "Plants",
            Self::Toys => "Toys",
            Self::Food => "Snacks",
        }
    }
}

/// The sets furniture and finishes come in: the house's own, always there, and three that are
/// made to go together and arrive with time, each in its own colours and motifs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Set {
    Home,
    Seaside,
    Woodland,
    Starlit,
}

impl Set {
    pub const ALL: [Self; 4] = [Self::Home, Self::Seaside, Self::Woodland, Self::Starlit];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Seaside => "Seaside",
            Self::Woodland => "Woodland",
            Self::Starlit => "Starlit",
        }
    }

    /// When a set's pieces and finishes arrive: slowly, a set at a time.
    pub const fn arrives(self) -> Arrival {
        match self {
            Self::Home => Arrival::Always,
            Self::Seaside => Arrival::AfterDays(5),
            Self::Woodland => Arrival::AfterDays(12),
            Self::Starlit => Arrival::AfterDays(24),
        }
    }
}

impl Arrival {
    pub fn come(self, days_lived: u32, things: usize) -> bool {
        match self {
            Self::Always => true,
            Self::AfterDays(days) => days_lived >= days,
            Self::AfterFinds(finds) => things >= finds,
        }
    }
}

/// One piece of furniture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub id: &'static str,
    pub name: &'static str,
    pub family: Family,
    /// Tiles along the room's width and depth, unturned. The piece's front faces down the depth.
    pub size: (u8, u8),
    /// How tall it stands, in pixels: what decides whether it hides someone behind it.
    pub height: i32,
    /// A rug: walked over, and stood on by other pieces.
    pub flat: bool,
    pub uses: &'static [Use],
    pub surfaces: &'static [Surface],
    pub arrives: Arrival,
    /// How high someone sitting or lying on it is lifted off the floor, in pixels.
    pub lift: i32,
    pub set: Set,
}

impl Piece {
    pub fn catalog_id(&self) -> CatalogId {
        CatalogId::known(self.id)
    }

    /// Its footprint at `turn`: a quarter turn swaps width and depth.
    pub fn size_at(&self, turn: u8) -> (u8, u8) {
        if turn % 2 == 1 {
            (self.size.1, self.size.0)
        } else {
            self.size
        }
    }

    pub fn has(&self, wanted: Use) -> bool {
        self.uses.iter().any(|own| match (own, wanted) {
            (Use::Sit { .. }, Use::Sit { .. }) => true,
            (own, wanted) => *own == wanted,
        })
    }

    pub fn seats(&self) -> u8 {
        self.uses
            .iter()
            .find_map(|own| match own {
                Use::Sit { seats } => Some(*seats),
                Use::Nap | Use::Sleep => Some(1),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// Tall enough to hide a companion standing behind it.
    pub fn tall(&self) -> bool {
        self.height >= 30
    }

    pub fn available(&self, days_lived: u32, things: usize) -> bool {
        self.arrives.come(days_lived, things)
    }

    /// For a light, how high its bulb glows above the floor, in pixels.
    pub fn glow(&self) -> Option<i32> {
        match self.id {
            "lamp" => Some(33),
            "shell_lamp" => Some(21),
            "mushroom_lamp" => Some(17),
            "moon_lamp" => Some(28),
            _ => (self.family == Family::Lights).then_some(self.height / 2),
        }
    }
}

/// A point on a piece's unturned footprint, moved to where it is at `turn`. The second and fourth
/// turns are the first and third seen in a mirror: every piece is the same on both sides of its
/// front, so a mirror shows it turned.
pub fn turned(point: (f32, f32), size: (u8, u8), turn: u8) -> (f32, f32) {
    let (w, d) = (f32::from(size.0), f32::from(size.1));
    let (x, y) = point;
    match turn % 4 {
        0 => (x, y),
        1 => (y, x),
        2 => (w - x, d - y),
        _ => (d - y, w - x),
    }
}

/// The way a piece's front faces at `turn`, as a step across the floor.
pub fn front(turn: u8) -> (i32, i32) {
    match turn % 4 {
        0 => (0, 1),
        1 => (1, 0),
        2 => (0, -1),
        _ => (-1, 0),
    }
}

const SEAT: [Use; 2] = [Use::Sit { seats: 1 }, Use::Nap];

pub static PIECES: [Piece; 31] = [
    Piece {
        id: "cushion",
        name: "Floor cushion",
        family: Family::Seats,
        size: (1, 1),
        height: 7,
        flat: false,
        uses: &SEAT,
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 4,
        set: Set::Home,
    },
    Piece {
        id: "armchair",
        name: "Armchair",
        family: Family::Seats,
        size: (1, 1),
        height: 28,
        flat: false,
        uses: &SEAT,
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 9,
        set: Set::Home,
    },
    Piece {
        id: "sofa",
        name: "Little sofa",
        family: Family::Seats,
        size: (2, 1),
        height: 28,
        flat: false,
        uses: &[Use::Sit { seats: 2 }, Use::Nap],
        surfaces: &[],
        arrives: Arrival::AfterDays(3),
        lift: 9,
        set: Set::Home,
    },
    Piece {
        id: "bed",
        name: "Little bed",
        family: Family::Beds,
        size: (1, 2),
        height: 16,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 6,
        set: Set::Home,
    },
    Piece {
        id: "basket",
        name: "Nest basket",
        family: Family::Beds,
        size: (1, 1),
        height: 9,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 3,
        set: Set::Home,
    },
    Piece {
        id: "side_table",
        name: "Side table",
        family: Family::Tables,
        size: (1, 1),
        height: 15,
        flat: false,
        uses: &[],
        surfaces: &[Surface {
            at: (0.5, 0.5),
            height: 15,
            holds: Holds::Top,
        }],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "low_table",
        name: "Low table",
        family: Family::Tables,
        size: (2, 1),
        height: 11,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.55, 0.5),
                height: 11,
                holds: Holds::Top,
            },
            Surface {
                at: (1.45, 0.5),
                height: 11,
                holds: Holds::Top,
            },
        ],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "shelf",
        name: "Open shelf",
        family: Family::Shelves,
        size: (1, 1),
        height: 44,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.5, 0.5),
                height: 4,
                holds: Holds::Shelf,
            },
            Surface {
                at: (0.5, 0.5),
                height: 18,
                holds: Holds::Shelf,
            },
            Surface {
                at: (0.5, 0.5),
                height: 32,
                holds: Holds::Shelf,
            },
        ],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "case",
        name: "Glass case",
        family: Family::Shelves,
        size: (1, 1),
        height: 36,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.5, 0.5),
                height: 12,
                holds: Holds::Shelf,
            },
            Surface {
                at: (0.5, 0.5),
                height: 25,
                holds: Holds::Shelf,
            },
        ],
        arrives: Arrival::AfterFinds(6),
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "round_rug",
        name: "Round rug",
        family: Family::Rugs,
        size: (2, 2),
        height: 0,
        flat: true,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "long_rug",
        name: "Long rug",
        family: Family::Rugs,
        size: (3, 2),
        height: 0,
        flat: true,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::AfterDays(7),
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "lamp",
        name: "Floor lamp",
        family: Family::Lights,
        size: (1, 1),
        height: 46,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "fern",
        name: "Potted fern",
        family: Family::Plants,
        size: (1, 1),
        height: 26,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "ball",
        name: "Bouncy ball",
        family: Family::Toys,
        size: (1, 1),
        height: 9,
        flat: false,
        uses: &[Use::Play],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "toy_box",
        name: "Toy box",
        family: Family::Toys,
        size: (1, 1),
        height: 22,
        flat: false,
        uses: &[Use::Play],
        surfaces: &[],
        arrives: Arrival::AfterDays(10),
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "snack_bowl",
        name: "Snack bowl",
        family: Family::Food,
        size: (1, 1),
        height: 6,
        flat: false,
        uses: &[Use::Snack],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 0,
        set: Set::Home,
    },
    // For showing things.
    Piece {
        id: "cabinet",
        name: "Curio cabinet",
        family: Family::Shelves,
        size: (1, 1),
        height: 54,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.5, 0.5),
                height: 8,
                holds: Holds::Shelf,
            },
            Surface {
                at: (0.5, 0.5),
                height: 22,
                holds: Holds::Shelf,
            },
            Surface {
                at: (0.5, 0.5),
                height: 36,
                holds: Holds::Shelf,
            },
        ],
        arrives: Arrival::AfterFinds(10),
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "plinth",
        name: "Bell jar",
        family: Family::Shelves,
        size: (1, 1),
        height: 34,
        flat: false,
        uses: &[],
        surfaces: &[Surface {
            at: (0.5, 0.5),
            height: 20,
            holds: Holds::Shelf,
        }],
        arrives: Arrival::AfterFinds(4),
        lift: 0,
        set: Set::Home,
    },
    Piece {
        id: "counter",
        name: "Glass counter",
        family: Family::Shelves,
        size: (2, 1),
        height: 20,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.42, 0.5),
                height: 10,
                holds: Holds::Shelf,
            },
            Surface {
                at: (1.0, 0.5),
                height: 10,
                holds: Holds::Shelf,
            },
            Surface {
                at: (1.58, 0.5),
                height: 10,
                holds: Holds::Shelf,
            },
        ],
        arrives: Arrival::AfterFinds(14),
        lift: 0,
        set: Set::Home,
    },
    // The seaside set.
    Piece {
        id: "deckchair",
        name: "Deckchair",
        family: Family::Seats,
        size: (1, 1),
        height: 22,
        flat: false,
        uses: &SEAT,
        surfaces: &[],
        arrives: Set::Seaside.arrives(),
        lift: 5,
        set: Set::Seaside,
    },
    Piece {
        id: "driftwood_bed",
        name: "Driftwood bed",
        family: Family::Beds,
        size: (1, 2),
        height: 16,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Set::Seaside.arrives(),
        lift: 6,
        set: Set::Seaside,
    },
    Piece {
        id: "rope_rug",
        name: "Rope rug",
        family: Family::Rugs,
        size: (2, 2),
        height: 0,
        flat: true,
        uses: &[],
        surfaces: &[],
        arrives: Set::Seaside.arrives(),
        lift: 0,
        set: Set::Seaside,
    },
    Piece {
        id: "shell_lamp",
        name: "Shell lamp",
        family: Family::Lights,
        size: (1, 1),
        height: 30,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Set::Seaside.arrives(),
        lift: 0,
        set: Set::Seaside,
    },
    // The woodland set.
    Piece {
        id: "toadstool",
        name: "Toadstool",
        family: Family::Seats,
        size: (1, 1),
        height: 13,
        flat: false,
        uses: &SEAT,
        surfaces: &[],
        arrives: Set::Woodland.arrives(),
        lift: 11,
        set: Set::Woodland,
    },
    Piece {
        id: "moss_bed",
        name: "Moss bed",
        family: Family::Beds,
        size: (1, 2),
        height: 16,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Set::Woodland.arrives(),
        lift: 6,
        set: Set::Woodland,
    },
    Piece {
        id: "log_table",
        name: "Log table",
        family: Family::Tables,
        size: (1, 1),
        height: 12,
        flat: false,
        uses: &[],
        surfaces: &[Surface {
            at: (0.5, 0.5),
            height: 12,
            holds: Holds::Top,
        }],
        arrives: Set::Woodland.arrives(),
        lift: 0,
        set: Set::Woodland,
    },
    Piece {
        id: "mushroom_lamp",
        name: "Mushroom lamp",
        family: Family::Lights,
        size: (1, 1),
        height: 24,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Set::Woodland.arrives(),
        lift: 0,
        set: Set::Woodland,
    },
    // The starlit set.
    Piece {
        id: "night_bed",
        name: "Night-sky bed",
        family: Family::Beds,
        size: (1, 2),
        height: 16,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Set::Starlit.arrives(),
        lift: 6,
        set: Set::Starlit,
    },
    Piece {
        id: "star_rug",
        name: "Star rug",
        family: Family::Rugs,
        size: (2, 2),
        height: 0,
        flat: true,
        uses: &[],
        surfaces: &[],
        arrives: Set::Starlit.arrives(),
        lift: 0,
        set: Set::Starlit,
    },
    Piece {
        id: "moon_lamp",
        name: "Moon lamp",
        family: Family::Lights,
        size: (1, 1),
        height: 36,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Set::Starlit.arrives(),
        lift: 0,
        set: Set::Starlit,
    },
    Piece {
        id: "telescope",
        name: "Toy telescope",
        family: Family::Toys,
        size: (1, 1),
        height: 30,
        flat: false,
        uses: &[Use::Play],
        surfaces: &[],
        arrives: Set::Starlit.arrives(),
        lift: 0,
        set: Set::Starlit,
    },
];

pub fn piece(id: &CatalogId) -> Option<&'static Piece> {
    PIECES.iter().find(|piece| piece.id == id.as_str())
}

/// A floor or a wall the owner can choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finish {
    pub id: &'static str,
    pub name: &'static str,
    pub set: Set,
}

pub static FLOORS: [Finish; 7] = [
    Finish {
        id: "floor.boards",
        name: "Honey boards",
        set: Set::Home,
    },
    Finish {
        id: "floor.checks",
        name: "Sage checks",
        set: Set::Home,
    },
    Finish {
        id: "floor.straw",
        name: "Woven straw",
        set: Set::Home,
    },
    Finish {
        id: "floor.rose",
        name: "Rose carpet",
        set: Set::Home,
    },
    Finish {
        id: "floor.sand",
        name: "Driftwood boards",
        set: Set::Seaside,
    },
    Finish {
        id: "floor.moss",
        name: "Moss carpet",
        set: Set::Woodland,
    },
    Finish {
        id: "floor.night",
        name: "Midnight tiles",
        set: Set::Starlit,
    },
];

pub static WALLS: [Finish; 7] = [
    Finish {
        id: "wall.plaster",
        name: "Cream plaster",
        set: Set::Home,
    },
    Finish {
        id: "wall.stripes",
        name: "Mint stripes",
        set: Set::Home,
    },
    Finish {
        id: "wall.leafy",
        name: "Leafy paper",
        set: Set::Home,
    },
    Finish {
        id: "wall.timber",
        name: "Timber",
        set: Set::Home,
    },
    Finish {
        id: "wall.waves",
        name: "Sea stripes",
        set: Set::Seaside,
    },
    Finish {
        id: "wall.birch",
        name: "Birch panels",
        set: Set::Woodland,
    },
    Finish {
        id: "wall.stars",
        name: "Starry paper",
        set: Set::Starlit,
    },
];

/// A kind of room a house can grow: its size, and the few pieces it comes with, placed in its own
/// tiles as `(piece, x, y, turn)`. The room's finishes start as the first room's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub size: (u8, u8),
    pub pieces: &'static [(&'static str, u8, u8, u8)],
}

/// Every kind of room a house can grow. Rooms come slowly, with the days a colony has lived:
/// see [`rooms_allowed`].
pub static ROOMS: [Template; 4] = [
    Template {
        id: "room.nook",
        name: "Reading nook",
        size: (4, 4),
        pieces: &[("lamp", 0, 0, 0), ("armchair", 0, 2, 1), ("shelf", 2, 0, 0)],
    },
    Template {
        id: "room.bedroom",
        name: "Bedroom",
        size: (5, 5),
        pieces: &[
            ("bed", 1, 0, 0),
            ("side_table", 2, 0, 0),
            ("basket", 4, 0, 0),
            ("round_rug", 2, 2, 0),
        ],
    },
    Template {
        id: "room.gallery",
        name: "Gallery",
        size: (4, 7),
        pieces: &[
            ("cabinet", 0, 1, 1),
            ("case", 0, 3, 1),
            ("counter", 1, 0, 0),
            ("shelf", 3, 0, 0),
            ("plinth", 2, 4, 0),
        ],
    },
    Template {
        id: "room.playroom",
        name: "Playroom",
        size: (6, 5),
        pieces: &[
            ("toy_box", 0, 0, 0),
            ("long_rug", 2, 2, 0),
            ("ball", 4, 1, 0),
        ],
    },
];

pub fn template(id: &CatalogId) -> Option<&'static Template> {
    ROOMS.iter().find(|template| template.id == id.as_str())
}

/// How many rooms a house may have after the colony has lived `days` days: a second room after
/// two weeks, a third after forty days. Nothing hurries it, and a house never has to grow.
pub fn rooms_allowed(days: u32) -> usize {
    1 + usize::from(days >= 14) + usize::from(days >= 40)
}

/// What a room in a house is called: by its kind, or, for the room the house opens into, the
/// front room.
pub fn room_name(kind: Option<&CatalogId>, first: bool) -> &'static str {
    match kind.and_then(template) {
        Some(template) => template.name,
        None if first => "Front room",
        None => "Room",
    }
}

/// How a house first looks inside, taking a hint from its outside.
pub fn finishes_for(style: HouseStyle) -> (&'static str, &'static str) {
    match style {
        HouseStyle::Tent => ("floor.straw", "wall.stripes"),
        HouseStyle::Mushroom => ("floor.rose", "wall.plaster"),
        HouseStyle::PillowFort => ("floor.checks", "wall.stripes"),
        HouseStyle::LeafHouse | HouseStyle::Unknown => ("floor.boards", "wall.leafy"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_identifier_is_an_identifier_and_names_one_thing() {
        let ids: Vec<&str> = PIECES
            .iter()
            .map(|piece| piece.id)
            .chain(FLOORS.iter().map(|finish| finish.id))
            .chain(WALLS.iter().map(|finish| finish.id))
            .collect();
        for id in &ids {
            assert!(CatalogId::parse(id).is_some(), "{id}");
        }
        let unique: BTreeSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn the_starter_catalogue_furnishes_a_room() {
        let starter: Vec<_> = PIECES
            .iter()
            .filter(|piece| piece.available(0, 0))
            .collect();
        assert!((10..=15).contains(&starter.len()), "{}", starter.len());
        for wanted in [Use::Sit { seats: 1 }, Use::Sleep, Use::Play, Use::Snack] {
            assert!(starter.iter().any(|piece| piece.has(wanted)), "{wanted:?}");
        }
        assert!(starter.iter().any(|piece| !piece.surfaces.is_empty()));
    }

    #[test]
    fn surfaces_stay_on_their_piece_however_it_is_turned() {
        for piece in &PIECES {
            for turn in 0..4 {
                let (w, d) = piece.size_at(turn);
                for surface in piece.surfaces {
                    let (x, y) = turned(surface.at, piece.size, turn);
                    assert!(
                        (0.0..=f32::from(w)).contains(&x) && (0.0..=f32::from(d)).contains(&y),
                        "{} turn {turn}",
                        piece.id
                    );
                }
            }
        }
    }

    #[test]
    fn every_kind_of_room_comes_with_pieces_that_fit_it() {
        for template in &ROOMS {
            assert!(CatalogId::parse(template.id).is_some(), "{}", template.id);
            let mut taken: Vec<(u8, u8)> = Vec::new();
            for &(id, x, y, turn) in template.pieces {
                let piece = PIECES
                    .iter()
                    .find(|piece| piece.id == id)
                    .unwrap_or_else(|| panic!("{}: {id}", template.id));
                let (w, d) = piece.size_at(turn);
                assert!(
                    x + w <= template.size.0 && y + d <= template.size.1,
                    "{}: {id} sticks out",
                    template.id
                );
                if piece.flat {
                    continue;
                }
                for ty in y..y + d {
                    for tx in x..x + w {
                        assert!(!taken.contains(&(tx, ty)), "{}: {id}", template.id);
                        taken.push((tx, ty));
                    }
                }
            }
        }
        assert_eq!(rooms_allowed(0), 1);
        assert_eq!(rooms_allowed(40), formiga_home_contract::limits::MAX_ROOMS);
    }

    #[test]
    fn every_set_comes_with_furniture_and_a_floor_and_a_wall_and_every_light_glows() {
        for set in Set::ALL {
            assert!(PIECES.iter().any(|piece| piece.set == set), "{set:?}");
            assert!(FLOORS.iter().any(|finish| finish.set == set), "{set:?}");
            assert!(WALLS.iter().any(|finish| finish.set == set), "{set:?}");
            // A set's pieces come with it, and none before the set does.
            for piece in PIECES
                .iter()
                .filter(|piece| piece.set == set && set != Set::Home)
            {
                assert_eq!(piece.arrives, set.arrives(), "{}", piece.id);
            }
        }
        for piece in PIECES.iter().filter(|piece| piece.family == Family::Lights) {
            let glow = piece
                .glow()
                .unwrap_or_else(|| panic!("{} gives no light", piece.id));
            assert!((4..piece.height).contains(&glow), "{}", piece.id);
        }
    }

    #[test]
    fn every_house_style_opens_onto_finishes_the_catalogue_has() {
        for style in [
            HouseStyle::Tent,
            HouseStyle::Mushroom,
            HouseStyle::PillowFort,
            HouseStyle::LeafHouse,
            HouseStyle::Unknown,
        ] {
            let (floor, wall) = finishes_for(style);
            assert!(FLOORS.iter().any(|finish| finish.id == floor));
            assert!(WALLS.iter().any(|finish| finish.id == wall));
        }
    }
}
