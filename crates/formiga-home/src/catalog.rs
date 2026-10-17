//! Everything Home can put in a room, by the identifier a layout keeps.
//!
//! Nothing here costs anything. The starter pieces are there from the first visit; a few more
//! arrive as the colony lives, or as it finds things worth a case, and a few are made by the
//! household itself, as it finishes its projects. Once they have arrived they stay. An identifier is never reused for something else, so a layout always means what it meant
//! when it was arranged; a piece a newer Home has, and this one does not, is kept in its place and
//! left alone.

use formiga_home_contract::{CatalogId, HomeSnapshot, HouseStyle, HouseholdHome, Project};

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
    /// What is over it, if anything, keeping what is shown there to its height.
    pub cover: Option<Cover>,
}

/// What is over a surface, and how far above it: nothing taller than that is shown there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cover {
    /// A board or a roof. What is shown stands at its front, where the board hides least of it.
    Board(i32),
    /// Glass, seen through.
    Glass(i32),
}

impl Cover {
    /// How much room there is under it, in pixels.
    pub fn clearance(self) -> i32 {
        match self {
            Self::Board(room) | Self::Glass(room) => room,
        }
    }
}

/// When a piece is in the catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrival {
    Always,
    /// Once the colony has lived this many days.
    AfterDays(u32),
    /// Once the colony has this many things to show.
    AfterFinds(usize),
    /// Once the household has finished this many stages of one of its projects.
    Made(&'static str, usize),
}

/// What has come to one household so far, which decides what it has to use: the days the colony
/// has lived, the things it has to show, and the stages of its projects the household has made.
#[derive(Clone, Copy, Debug)]
pub struct Arrived<'a> {
    pub days: u32,
    pub things: usize,
    pub made: &'a [Project],
}

impl<'a> Arrived<'a> {
    pub fn of(snapshot: &HomeSnapshot, home: &'a HouseholdHome) -> Self {
        Self {
            days: snapshot.days_lived,
            things: crate::keepsakes::colony_things(snapshot).count(),
            made: &home.projects,
        }
    }

    /// The very start: nothing lived, found or made.
    #[cfg(test)]
    pub const FIRST: Arrived<'static> = Arrived {
        days: 0,
        things: 0,
        made: &[],
    };
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

/// The sets furniture and finishes come in: the house's own, always there; three that are made
/// to go together and arrive with time, each in its own colours and motifs; and what the
/// household makes for itself, piece by piece, as it finishes its projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Set {
    Home,
    Seaside,
    Woodland,
    Starlit,
    Made,
}

impl Set {
    pub const ALL: [Self; 5] = [
        Self::Home,
        Self::Seaside,
        Self::Woodland,
        Self::Starlit,
        Self::Made,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Seaside => "Seaside",
            Self::Woodland => "Woodland",
            Self::Starlit => "Starlit",
            Self::Made => "Made at home",
        }
    }

    /// When a set's pieces and finishes arrive: slowly, a set at a time. What the household
    /// makes arrives piece by piece, each as its project's stage is finished.
    pub const fn arrives(self) -> Arrival {
        match self {
            Self::Home | Self::Made => Arrival::Always,
            Self::Seaside => Arrival::AfterDays(5),
            Self::Woodland => Arrival::AfterDays(12),
            Self::Starlit => Arrival::AfterDays(24),
        }
    }
}

impl Arrival {
    pub fn come(self, now: &Arrived) -> bool {
        match self {
            Self::Always => true,
            Self::AfterDays(days) => now.days >= days,
            Self::AfterFinds(finds) => now.things >= finds,
            Self::Made(project, stages) => now
                .made
                .iter()
                .any(|made| made.project.as_str() == project && made.stages.len() >= stages),
        }
    }

    /// When it comes, for the owner, if it has not come yet: "After 5 more days lived", "With 2
    /// more finds", "Made by finishing In bloom, for the window garden".
    pub fn when(self, now: &Arrived) -> Option<String> {
        if self.come(now) {
            return None;
        }
        Some(match self {
            Self::Always => return None,
            Self::AfterDays(days) => match days.saturating_sub(now.days) {
                1 => "After one more day lived".to_owned(),
                left => format!("After {left} more days lived"),
            },
            Self::AfterFinds(finds) => match finds.saturating_sub(now.things) {
                1 => "Once the colony has one more find".to_owned(),
                left => format!("Once the colony has {left} more finds"),
            },
            Self::Made(id, stage) => match crate::projects::project(id)
                .and_then(|project| Some((project, project.stages.get(stage.checked_sub(1)?)?)))
            {
                Some((project, stage)) => {
                    format!("Made by finishing {}, for {}", stage.name, project.the())
                }
                None => "Made by finishing a project".to_owned(),
            },
        })
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

    pub fn available(&self, now: &Arrived) -> bool {
        self.arrives.come(now)
    }

    /// For a light, how high its bulb glows above the floor, in pixels.
    pub fn glow(&self) -> Option<i32> {
        match self.id {
            "lamp" => Some(33),
            "shell_lamp" => Some(21),
            "mushroom_lamp" => Some(17),
            "moon_lamp" => Some(28),
            "hearth" | "hearth_mantel" | "hearth_garland" => Some(10),
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

/// The low table's top, two places along it: a table made from it keeps them.
const LOW_TABLE_TOP: [Surface; 2] = [
    Surface {
        at: (0.55, 0.5),
        height: 11,
        holds: Holds::Top,
        cover: None,
    },
    Surface {
        at: (1.45, 0.5),
        height: 11,
        holds: Holds::Top,
        cover: None,
    },
];

/// A mantel over a hearth: two places on its shelf, either side of the chimney breast.
const MANTEL: [Surface; 2] = [
    Surface {
        at: (0.42, 0.3),
        height: 34,
        holds: Holds::Shelf,
        cover: None,
    },
    Surface {
        at: (1.58, 0.3),
        height: 34,
        holds: Holds::Shelf,
        cover: None,
    },
];

pub static PIECES: [Piece; 44] = [
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
        id: "bedroll",
        name: "Guest bedroll",
        family: Family::Beds,
        size: (1, 2),
        height: 6,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Always,
        lift: 2,
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
            cover: None,
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
        surfaces: &LOW_TABLE_TOP,
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
                cover: Some(Cover::Board(18)),
            },
            Surface {
                at: (0.5, 0.5),
                height: 24,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(18)),
            },
            Surface {
                at: (0.5, 0.5),
                height: 44,
                holds: Holds::Shelf,
                cover: None,
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
        height: 44,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.5, 0.5),
                height: 6,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(17)),
            },
            Surface {
                at: (0.5, 0.5),
                height: 24,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(17)),
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
        height: 55,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.5, 0.5),
                height: 4,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(15)),
            },
            Surface {
                at: (0.5, 0.5),
                height: 20,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(15)),
            },
            Surface {
                at: (0.5, 0.5),
                height: 36,
                holds: Holds::Shelf,
                cover: Some(Cover::Board(15)),
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
            cover: Some(Cover::Glass(15)),
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
        height: 26,
        flat: false,
        uses: &[],
        surfaces: &[
            Surface {
                at: (0.3, 0.5),
                height: 10,
                holds: Holds::Shelf,
                cover: Some(Cover::Glass(15)),
            },
            Surface {
                at: (1.0, 0.5),
                height: 10,
                holds: Holds::Shelf,
                cover: Some(Cover::Glass(15)),
            },
            Surface {
                at: (1.7, 0.5),
                height: 10,
                holds: Holds::Shelf,
                cover: Some(Cover::Glass(15)),
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
            cover: None,
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
    // What households make for themselves, as they finish their projects' stages. An improved
    // piece stands on the footprint of the one it was made from, with the same surfaces, so it
    // can be made in place with whatever is on it.
    Piece {
        id: "window_box",
        name: "Window box",
        family: Family::Plants,
        size: (2, 1),
        height: 15,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Made("project.window_garden", 1),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "window_box_bloom",
        name: "Window box in bloom",
        family: Family::Plants,
        size: (2, 1),
        height: 22,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Made("project.window_garden", 2),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "puppet_theatre",
        name: "Puppet theatre",
        family: Family::Toys,
        size: (2, 1),
        height: 40,
        flat: false,
        uses: &[Use::Play],
        surfaces: &[],
        arrives: Arrival::Made("project.puppet_theatre", 1),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "puppet_theatre_fair",
        name: "Fairground theatre",
        family: Family::Toys,
        size: (2, 1),
        height: 44,
        flat: false,
        uses: &[Use::Play],
        surfaces: &[],
        arrives: Arrival::Made("project.puppet_theatre", 2),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "quilt_bed",
        name: "Quilted bed",
        family: Family::Beds,
        size: (1, 2),
        height: 16,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Made("project.cosy_beds", 1),
        lift: 6,
        set: Set::Made,
    },
    Piece {
        id: "feather_bed",
        name: "Feather bed",
        family: Family::Beds,
        size: (1, 2),
        height: 18,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Made("project.cosy_beds", 2),
        lift: 7,
        set: Set::Made,
    },
    Piece {
        id: "canopy_bed",
        name: "Canopy bed",
        family: Family::Beds,
        size: (1, 2),
        height: 44,
        flat: false,
        uses: &[Use::Sleep, Use::Nap],
        surfaces: &[],
        arrives: Arrival::Made("project.cosy_beds", 3),
        lift: 7,
        set: Set::Made,
    },
    Piece {
        id: "gingham_table",
        name: "Gingham table",
        family: Family::Tables,
        size: (2, 1),
        height: 11,
        flat: false,
        uses: &[],
        surfaces: &LOW_TABLE_TOP,
        arrives: Arrival::Made("project.family_table", 1),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "harvest_table",
        name: "Harvest table",
        family: Family::Tables,
        size: (2, 1),
        height: 11,
        flat: false,
        uses: &[],
        surfaces: &LOW_TABLE_TOP,
        arrives: Arrival::Made("project.family_table", 2),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "hearth",
        name: "Hearth",
        family: Family::Lights,
        size: (2, 1),
        height: 36,
        flat: false,
        uses: &[],
        surfaces: &[],
        arrives: Arrival::Made("project.hearth", 1),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "hearth_mantel",
        name: "Hearth with a mantel",
        family: Family::Lights,
        size: (2, 1),
        height: 40,
        flat: false,
        uses: &[],
        surfaces: &MANTEL,
        arrives: Arrival::Made("project.hearth", 2),
        lift: 0,
        set: Set::Made,
    },
    Piece {
        id: "hearth_garland",
        name: "Household hearth",
        family: Family::Lights,
        size: (2, 1),
        height: 40,
        flat: false,
        uses: &[],
        surfaces: &MANTEL,
        arrives: Arrival::Made("project.hearth", 3),
        lift: 0,
        set: Set::Made,
    },
];

pub fn piece(id: &CatalogId) -> Option<&'static Piece> {
    PIECES.iter().find(|piece| piece.id == id.as_str())
}

/// A floor or a wall, by its identifier.
pub fn finish(id: &str) -> Option<&'static Finish> {
    FLOORS.iter().chain(&WALLS).find(|finish| finish.id == id)
}

/// A floor or a wall the owner can choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finish {
    pub id: &'static str,
    pub name: &'static str,
    pub set: Set,
    pub arrives: Arrival,
}

impl Finish {
    pub fn available(&self, now: &Arrived) -> bool {
        self.arrives.come(now)
    }
}

pub static FLOORS: [Finish; 9] = [
    Finish {
        id: "floor.boards",
        name: "Honey boards",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "floor.checks",
        name: "Sage checks",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "floor.straw",
        name: "Woven straw",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "floor.rose",
        name: "Rose carpet",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "floor.sand",
        name: "Driftwood boards",
        set: Set::Seaside,
        arrives: Set::Seaside.arrives(),
    },
    Finish {
        id: "floor.moss",
        name: "Moss carpet",
        set: Set::Woodland,
        arrives: Set::Woodland.arrives(),
    },
    Finish {
        id: "floor.night",
        name: "Midnight tiles",
        set: Set::Starlit,
        arrives: Set::Starlit.arrives(),
    },
    Finish {
        id: "floor.parquet",
        name: "Oak parquet",
        set: Set::Home,
        arrives: Arrival::AfterDays(4),
    },
    Finish {
        id: "floor.hearthstone",
        name: "Hearthstone",
        set: Set::Made,
        arrives: Arrival::Made("project.hearth", 1),
    },
];

pub static WALLS: [Finish; 10] = [
    Finish {
        id: "wall.plaster",
        name: "Cream plaster",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "wall.stripes",
        name: "Mint stripes",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "wall.leafy",
        name: "Leafy paper",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "wall.timber",
        name: "Timber",
        set: Set::Home,
        arrives: Set::Home.arrives(),
    },
    Finish {
        id: "wall.waves",
        name: "Sea stripes",
        set: Set::Seaside,
        arrives: Set::Seaside.arrives(),
    },
    Finish {
        id: "wall.birch",
        name: "Birch panels",
        set: Set::Woodland,
        arrives: Set::Woodland.arrives(),
    },
    Finish {
        id: "wall.stars",
        name: "Starry paper",
        set: Set::Starlit,
        arrives: Set::Starlit.arrives(),
    },
    Finish {
        id: "wall.wainscot",
        name: "Wainscot",
        set: Set::Home,
        arrives: Arrival::AfterDays(4),
    },
    Finish {
        id: "wall.trellis",
        name: "Trellis paper",
        set: Set::Made,
        arrives: Arrival::Made("project.window_garden", 2),
    },
    Finish {
        id: "wall.gingham",
        name: "Gingham paper",
        set: Set::Made,
        arrives: Arrival::Made("project.family_table", 1),
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
    1 + ROOMS_COME.iter().filter(|&&on| days >= on).count()
}

/// The days lived after which a second room may be built, and a third.
const ROOMS_COME: [u32; 2] = [14, 40];

/// How many more days the colony has to live before a house of `rooms` rooms may build another,
/// if it ever may.
pub fn days_to_next_room(rooms: usize, days: u32) -> Option<u32> {
    let on = *ROOMS_COME.get(rooms.checked_sub(1)?)?;
    Some(on.saturating_sub(days))
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
            .filter(|piece| piece.available(&Arrived::FIRST))
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
    fn what_has_not_come_yet_says_how_far_off_it_is() {
        let now = Arrived {
            days: 7,
            things: 3,
            made: &[],
        };
        assert_eq!(
            Arrival::AfterDays(12).when(&now).as_deref(),
            Some("After 5 more days lived")
        );
        assert_eq!(
            Arrival::AfterDays(8).when(&now).as_deref(),
            Some("After one more day lived")
        );
        assert_eq!(
            Arrival::AfterFinds(6).when(&now).as_deref(),
            Some("Once the colony has 3 more finds")
        );
        assert_eq!(
            Arrival::Made("project.window_garden", 2)
                .when(&now)
                .as_deref(),
            Some("Made by finishing In bloom, for the window garden")
        );
        assert_eq!(Arrival::AfterDays(5).when(&now), None, "already come");
        assert_eq!(Arrival::Always.when(&now), None);
        // Every piece and finish not yet come says when it will.
        for arrives in PIECES
            .iter()
            .map(|piece| piece.arrives)
            .chain(FLOORS.iter().chain(&WALLS).map(|finish| finish.arrives))
        {
            let when = arrives.when(&Arrived::FIRST);
            assert_eq!(
                when.is_some(),
                !arrives.come(&Arrived::FIRST),
                "{arrives:?}"
            );
            assert!(
                when.is_none_or(|when| !when.contains("a project")),
                "{arrives:?} names its stage"
            );
        }
        assert_eq!(days_to_next_room(1, 3), Some(11));
        assert_eq!(days_to_next_room(2, 20), Some(20));
        assert_eq!(days_to_next_room(2, 50), Some(0));
        assert_eq!(days_to_next_room(3, 50), None);
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
                .filter(|piece| piece.set == set && !matches!(set, Set::Home | Set::Made))
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
