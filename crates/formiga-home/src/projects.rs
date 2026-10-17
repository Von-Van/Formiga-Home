//! Household projects: things a household makes of its home over time, a stage at a time.
//!
//! Nothing here costs anything or runs down. A project's stage is ready when the household has
//! lived what it asks for: things that happen at home, counted as they happen; the days the
//! colony has lived and the keepsakes the house has; and souvenirs anyone in the colony has
//! brought home from Formiga Hill, which every household counts alike. What a household has made
//! is its own, kept with its home: a project finished in one house is not finished in another.
//!
//! The owner finishes a ready stage from the project board, whenever they like, and its gifts
//! arrive at once: a piece the household has made, standing in the house if it has room; pieces
//! already there made into something better; or a new floor or wall to choose. Every stage
//! finished is kept, with when, for good.

use crate::catalog::{self, Piece};
use crate::house::House;
use crate::placement;
use formiga_home_contract::limits::{MAX_PROJECTS, MAX_TALLIES};
use formiga_home_contract::{
    CatalogId, DisplaySource, HomeMoment, HomeSnapshot, HouseholdHome, Project as Record,
    StageDone, Tally,
};
use time::OffsetDateTime;

/// The three kinds of project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Building something the house did not have.
    Construction,
    /// Making something the house has into something better, a stage at a time.
    Improvement,
    /// Marking the household's own history with something that stays.
    Legacy,
}

impl Kind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Construction => "Construction",
            Self::Improvement => "Improvement",
            Self::Legacy => "Legacy",
        }
    }
}

/// Something that happens at home and is counted for the household's projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// A snack eaten, or shared.
    Snacks,
    /// A nap, or a night's sleep.
    Rests,
    /// A game played, alone or together.
    Games,
    /// A plant tended.
    Tending,
    /// A find looked over, remembered or shown to someone.
    Looks,
    /// A friend come over.
    Visits,
    /// A friend who stayed over.
    Sleepovers,
    /// A while spent warming up by the hearth.
    Hearthside,
    /// A supper at the family table.
    Suppers,
    /// A show put on at the puppet theatre.
    Shows,
}

impl Count {
    #[cfg(test)]
    pub const ALL: [Self; 10] = [
        Self::Snacks,
        Self::Rests,
        Self::Games,
        Self::Tending,
        Self::Looks,
        Self::Visits,
        Self::Sleepovers,
        Self::Hearthside,
        Self::Suppers,
        Self::Shows,
    ];

    /// How the household's home keeps it.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Snacks => "tally.snacks",
            Self::Rests => "tally.rests",
            Self::Games => "tally.games",
            Self::Tending => "tally.tending",
            Self::Looks => "tally.looks",
            Self::Visits => "tally.visits",
            Self::Sleepovers => "tally.sleepovers",
            Self::Hearthside => "tally.hearthside",
            Self::Suppers => "tally.suppers",
            Self::Shows => "tally.shows",
        }
    }

    /// What it is, counted.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Snacks => "Snacks shared",
            Self::Rests => "Naps and nights' sleep",
            Self::Games => "Games played",
            Self::Tending => "Plants tended",
            Self::Looks => "Finds looked over",
            Self::Visits => "Friends come over",
            Self::Sleepovers => "Friends stayed over",
            Self::Hearthside => "Evenings by the hearth",
            Self::Suppers => "Suppers at the family table",
            Self::Shows => "Puppet shows put on",
        }
    }
}

/// What a stage asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Need {
    /// One of Formiga Hill's souvenirs, by Hill's own identifier, brought home by anyone in the
    /// colony.
    Souvenir(&'static str),
    /// Something that happens at home, this many times.
    Counted(Count, u32),
    /// The colony has lived this many days.
    Days(u32),
    /// The house has this many keepsakes of its own.
    Keepsakes(u32),
    /// One of these pieces stands in the house.
    Has(&'static [&'static str]),
    /// This many of Formiga Battle's keepsakes, brought home by anyone in the colony.
    BattleKeepsakes(u32),
    /// Any one of these will do: a souvenir from Formiga Hill, say, or more of what happens at
    /// home, so that no stage waits on Formiga Hill or on friends from other houses.
    AnyOf(&'static [Need]),
}

/// What finishing a stage gives the household.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gift {
    /// A piece the household has made. It stands in the house if there is room, and the
    /// furniture page has it from then on.
    Piece(&'static str),
    /// Every one of `from` in the house is made into `to`, where it stands; the furniture page
    /// has `to` from then on.
    Upgrade {
        from: &'static [&'static str],
        to: &'static str,
    },
    /// A floor or a wall to choose.
    Finish(&'static str),
}

/// One stage of a project.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stage {
    pub name: &'static str,
    /// What finishing it does, in a line.
    pub about: &'static str,
    pub needs: &'static [Need],
    pub gifts: &'static [Gift],
}

/// A project, by the identifier a home keeps it under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Project {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    pub about: &'static str,
    /// What the household must have before the project is on its board at all.
    pub shows: &'static [Need],
    pub stages: &'static [Stage],
}

/// Every bed a household might have, made cosier or not: what the cosy beds are for.
const BEDS: &[&str] = &[
    "bed",
    "quilt_bed",
    "feather_bed",
    "canopy_bed",
    "basket",
    "quilt_basket",
    "feather_basket",
    "driftwood_bed",
    "driftwood_quilt_bed",
    "driftwood_feather_bed",
    "driftwood_canopy_bed",
    "moss_bed",
    "moss_quilt_bed",
    "moss_feather_bed",
    "moss_canopy_bed",
    "night_bed",
    "night_quilt_bed",
    "night_feather_bed",
    "night_canopy_bed",
];
const TABLES: &[&str] = &["low_table", "gingham_table", "harvest_table"];

/// Every project a household can take on, in the order the board lists them. No stage waits on
/// Formiga Hill, or on friends from other houses: what one would bring can be lived at home
/// instead, a little more of it.
pub static PROJECTS: [Project; 6] = [
    Project {
        id: "project.window_garden",
        name: "Window garden",
        kind: Kind::Construction,
        about: "A planter of flowers like the ones on the Hill, for the household to tend.",
        shows: &[],
        stages: &[
            Stage {
                name: "Plant the window box",
                about: "A window box to stand under a window, with seedlings in it.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("pressed_daisy"),
                        Need::Counted(Count::Tending, 10),
                    ]),
                    Need::Counted(Count::Tending, 4),
                ],
                gifts: &[Gift::Piece("window_box")],
            },
            Stage {
                name: "In bloom",
                about: "The window box flowers, and a trellis paper comes for the walls.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("well_penny"),
                        Need::Counted(Count::Tending, 12),
                    ]),
                    Need::Days(10),
                ],
                gifts: &[
                    Gift::Upgrade {
                        from: &["window_box"],
                        to: "window_box_bloom",
                    },
                    Gift::Finish("wall.trellis"),
                ],
            },
        ],
    },
    Project {
        id: "project.puppet_theatre",
        name: "Puppet theatre",
        kind: Kind::Construction,
        about: "A little booth for putting on shows, after the stalls at the Fairground.",
        shows: &[],
        stages: &[
            Stage {
                name: "Build the booth",
                about: "A puppet theatre, to put on shows from, alone or for someone.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("hoopla_teddy"),
                        Need::Souvenir("chest_marble"),
                        Need::Counted(Count::Games, 14),
                    ]),
                    Need::Counted(Count::Games, 6),
                ],
                gifts: &[Gift::Piece("puppet_theatre")],
            },
            Stage {
                name: "Fairground curtains",
                about: "Striped curtains and bunting for the booth.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("fair_ticket"),
                        Need::Souvenir("race_rosette"),
                        Need::Souvenir("tug_rope"),
                        Need::Counted(Count::Games, 30),
                    ]),
                    Need::Counted(Count::Games, 16),
                    Need::AnyOf(&[
                        Need::Counted(Count::Visits, 2),
                        Need::Counted(Count::Shows, 6),
                    ]),
                ],
                gifts: &[Gift::Upgrade {
                    from: &["puppet_theatre"],
                    to: "puppet_theatre_fair",
                }],
            },
        ],
    },
    Project {
        id: "project.cosy_beds",
        name: "Cosy beds",
        kind: Kind::Improvement,
        about: "The house's beds, made a little cosier each time.",
        shows: &[Need::Has(BEDS)],
        stages: &[
            Stage {
                name: "Patchwork quilts",
                about: "Every bed in the house, and every nest basket, gets a patchwork quilt.",
                needs: &[Need::Counted(Count::Rests, 6)],
                gifts: &[
                    Gift::Upgrade {
                        from: &["bed"],
                        to: "quilt_bed",
                    },
                    Gift::Upgrade {
                        from: &["driftwood_bed"],
                        to: "driftwood_quilt_bed",
                    },
                    Gift::Upgrade {
                        from: &["moss_bed"],
                        to: "moss_quilt_bed",
                    },
                    Gift::Upgrade {
                        from: &["night_bed"],
                        to: "night_quilt_bed",
                    },
                    Gift::Upgrade {
                        from: &["basket"],
                        to: "quilt_basket",
                    },
                ],
            },
            Stage {
                name: "Feather pillows",
                about: "A second pillow in every bed and basket, plump with feathers.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("swing_feather"),
                        Need::Counted(Count::Rests, 14),
                    ]),
                    Need::Days(12),
                ],
                gifts: &[
                    Gift::Upgrade {
                        from: &["bed", "quilt_bed"],
                        to: "feather_bed",
                    },
                    Gift::Upgrade {
                        from: &["driftwood_bed", "driftwood_quilt_bed"],
                        to: "driftwood_feather_bed",
                    },
                    Gift::Upgrade {
                        from: &["moss_bed", "moss_quilt_bed"],
                        to: "moss_feather_bed",
                    },
                    Gift::Upgrade {
                        from: &["night_bed", "night_quilt_bed"],
                        to: "night_feather_bed",
                    },
                    Gift::Upgrade {
                        from: &["basket", "quilt_basket"],
                        to: "feather_basket",
                    },
                ],
            },
            Stage {
                name: "A canopy",
                about: "A canopy over each long bed, to remember sleeping under.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Counted(Count::Sleepovers, 1),
                        Need::Counted(Count::Rests, 30),
                    ]),
                    Need::Days(20),
                ],
                gifts: &[
                    Gift::Upgrade {
                        from: &["bed", "quilt_bed", "feather_bed"],
                        to: "canopy_bed",
                    },
                    Gift::Upgrade {
                        from: &[
                            "driftwood_bed",
                            "driftwood_quilt_bed",
                            "driftwood_feather_bed",
                        ],
                        to: "driftwood_canopy_bed",
                    },
                    Gift::Upgrade {
                        from: &["moss_bed", "moss_quilt_bed", "moss_feather_bed"],
                        to: "moss_canopy_bed",
                    },
                    Gift::Upgrade {
                        from: &["night_bed", "night_quilt_bed", "night_feather_bed"],
                        to: "night_canopy_bed",
                    },
                ],
            },
        ],
    },
    Project {
        id: "project.family_table",
        name: "Family table",
        kind: Kind::Improvement,
        about: "The low table, made the place the household eats together.",
        shows: &[Need::Has(TABLES)],
        stages: &[
            Stage {
                name: "A gingham cloth",
                about: "A gingham cloth like the picnic ribbon's, and a gingham paper for walls. \
                        The household has its suppers there from then on.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("picnic_ribbon"),
                        Need::Counted(Count::Snacks, 12),
                    ]),
                    Need::Counted(Count::Snacks, 5),
                ],
                gifts: &[
                    Gift::Upgrade {
                        from: &["low_table"],
                        to: "gingham_table",
                    },
                    Gift::Finish("wall.gingham"),
                ],
            },
            Stage {
                name: "Harvest supper",
                about: "A bowl of acorns and apples, a candle, and a brass bell like the high \
                        striker's, to ring when supper is ready.",
                needs: &[
                    Need::AnyOf(&[
                        Need::Souvenir("striker_bell"),
                        Need::Souvenir("oak_acorn"),
                        Need::Counted(Count::Snacks, 20),
                    ]),
                    Need::AnyOf(&[
                        Need::Counted(Count::Visits, 3),
                        Need::Counted(Count::Suppers, 6),
                    ]),
                ],
                gifts: &[Gift::Upgrade {
                    from: &["low_table", "gingham_table"],
                    to: "harvest_table",
                }],
            },
        ],
    },
    Project {
        id: "project.hearth",
        name: "The hearth",
        kind: Kind::Legacy,
        about: "The heart of the house, which grows with the household's own history.",
        shows: &[],
        stages: &[
            Stage {
                name: "Lay the hearth",
                about: "A stone hearth with a fire in it to warm up by, and hearthstone for the \
                        floors.",
                needs: &[Need::Days(7), Need::Keepsakes(1)],
                gifts: &[Gift::Piece("hearth"), Gift::Finish("floor.hearthstone")],
            },
            Stage {
                name: "A mantel for keepsakes",
                about: "A mantel over the fire, with room on it for what matters most.",
                needs: &[
                    Need::Days(21),
                    Need::Keepsakes(3),
                    Need::AnyOf(&[
                        Need::Counted(Count::Sleepovers, 1),
                        Need::Counted(Count::Hearthside, 6),
                    ]),
                ],
                gifts: &[Gift::Upgrade {
                    from: &["hearth"],
                    to: "hearth_mantel",
                }],
            },
            Stage {
                name: "The household's own",
                about: "A garland in the household's colours over the mantel, for good.",
                needs: &[
                    Need::Days(40),
                    Need::Keepsakes(6),
                    Need::AnyOf(&[
                        Need::Counted(Count::Visits, 6),
                        Need::Counted(Count::Hearthside, 15),
                    ]),
                ],
                gifts: &[Gift::Upgrade {
                    from: &["hearth", "hearth_mantel"],
                    to: "hearth_garland",
                }],
            },
        ],
    },
    Project {
        id: "project.podium",
        name: "Winners' podium",
        kind: Kind::Legacy,
        about: "Somewhere to stand the cups and medals the colony has won at Formiga Battle.",
        shows: &[Need::BattleKeepsakes(1)],
        stages: &[
            Stage {
                name: "Build the podium",
                about: "A podium of three steps, for a cup or a medal on each.",
                needs: &[Need::BattleKeepsakes(1), Need::Counted(Count::Looks, 6)],
                gifts: &[Gift::Piece("podium")],
            },
            Stage {
                name: "Bunting for the winners",
                about: "Bunting over the podium, in the colours of every cup.",
                needs: &[Need::BattleKeepsakes(3), Need::Counted(Count::Looks, 16)],
                gifts: &[Gift::Upgrade {
                    from: &["podium"],
                    to: "podium_bunting",
                }],
            },
        ],
    },
];

pub fn project(id: &str) -> Option<&'static Project> {
    PROJECTS.iter().find(|project| project.id == id)
}

/// What a household's projects are measured against: the visit's snapshot and its home.
#[derive(Clone, Copy)]
pub struct Facts<'a> {
    pub snapshot: &'a HomeSnapshot,
    pub home: &'a HouseholdHome,
}

/// How many times the household's home has counted `count`.
pub fn counted(home: &HouseholdHome, count: Count) -> u32 {
    home.tallies
        .iter()
        .find(|tally| tally.what.as_str() == count.id())
        .map_or(0, |tally| tally.count)
}

/// Count `count` once more for the household.
pub fn count(home: &mut HouseholdHome, count: Count) {
    bump(home, count.id(), 1);
}

fn bump(home: &mut HouseholdHome, id: &'static str, by: u32) {
    let full = home.tallies.len() >= MAX_TALLIES;
    match home
        .tallies
        .iter_mut()
        .find(|tally| tally.what.as_str() == id)
    {
        Some(tally) => tally.count = tally.count.saturating_add(by),
        None if !full => home.tallies.push(Tally {
            what: CatalogId::known(id),
            count: by,
        }),
        None => {}
    }
}

/// Whether anyone in the colony has brought the souvenir Formiga Hill calls `id` home.
pub fn has_souvenir(snapshot: &HomeSnapshot, id: &str) -> bool {
    snapshot
        .inventory
        .iter()
        .any(|item| matches!(&item.source, DisplaySource::HillSouvenir { id: kept } if kept == id))
}

/// Formiga Hill's souvenir `id`, by Desktop's name for it.
pub fn souvenir_name(id: &str) -> &'static str {
    formiga_core::Souvenir::from_id(id).map_or("a souvenir", |souvenir| souvenir.name())
}

/// How many of Formiga Battle's keepsakes anyone in the colony has brought home.
pub fn battle_keepsakes(snapshot: &HomeSnapshot) -> u32 {
    snapshot
        .inventory
        .iter()
        .filter(|item| matches!(item.source, DisplaySource::BattleKeepsake { .. }))
        .count() as u32
}

impl Need {
    /// How far the household has come towards it, and how far it asks. Of several that would
    /// each do, the one it is furthest along.
    pub fn progress(&self, facts: &Facts) -> (u32, u32) {
        let (have, want) = match *self {
            Self::Souvenir(id) => (u32::from(has_souvenir(facts.snapshot, id)), 1),
            Self::Counted(what, want) => (counted(facts.home, what), want),
            Self::Days(want) => (facts.snapshot.days_lived, want),
            Self::Keepsakes(want) => (facts.home.mementos.len() as u32, want),
            Self::Has(pieces) => {
                let has = facts.home.rooms.iter().any(|room| {
                    room.pieces
                        .iter()
                        .any(|placed| pieces.contains(&placed.piece.as_str()))
                });
                (u32::from(has), 1)
            }
            Self::BattleKeepsakes(want) => (battle_keepsakes(facts.snapshot), want),
            Self::AnyOf(needs) => needs
                .iter()
                .map(|need| need.progress(facts))
                .max_by(|(a, x), (b, y)| {
                    (f64::from(*a) / f64::from((*x).max(1)))
                        .total_cmp(&(f64::from(*b) / f64::from((*y).max(1))))
                })
                .unwrap_or((0, 1)),
        };
        (have.min(want), want)
    }

    pub fn met(&self, facts: &Facts) -> bool {
        match self {
            Self::AnyOf(needs) => needs.iter().any(|need| need.met(facts)),
            _ => {
                let (have, want) = self.progress(facts);
                have >= want
            }
        }
    }

    /// What it asks for, in a few words.
    pub fn label(&self) -> String {
        match *self {
            Self::Souvenir(id) => format!("{}, from Formiga Hill", souvenir_name(id)),
            Self::Counted(what, _) => what.label().to_owned(),
            Self::Days(days) => format!("{days} days lived"),
            Self::Keepsakes(_) => "Keepsakes made at home".to_owned(),
            Self::Has(pieces) => {
                let name = pieces
                    .first()
                    .and_then(|id| catalog::piece(&CatalogId::known(id)))
                    .map_or("A piece", |piece| piece.name);
                format!("{name} in the house")
            }
            Self::BattleKeepsakes(_) => "Cups and medals from Formiga Battle".to_owned(),
            Self::AnyOf(needs) => {
                let souvenirs: Vec<String> = needs
                    .iter()
                    .filter_map(|need| match need {
                        Self::Souvenir(id) => Some(souvenir_name(id).to_owned()),
                        _ => None,
                    })
                    .collect();
                let mut said = Vec::new();
                if !souvenirs.is_empty() {
                    said.push(format!("{} from Formiga Hill", or_list(&souvenirs)));
                }
                said.extend(
                    needs
                        .iter()
                        .filter(|need| !matches!(need, Self::Souvenir(_)))
                        .map(Need::label),
                );
                lower_after_first(said).join(", or ")
            }
        }
    }

    /// What it asks for and, where it is counted out and not yet met, how far along it is: "Plants
    /// tended: 3 of 4", "Pressed daisy from Formiga Hill, or plants tended: 3 of 10".
    pub fn line(&self, facts: &Facts) -> String {
        match self {
            // Met, it is whichever met it.
            Self::AnyOf(needs) if self.met(facts) => needs
                .iter()
                .find(|need| need.met(facts))
                .map_or_else(|| self.label(), |need| need.line(facts)),
            Self::AnyOf(needs) if !self.met(facts) => {
                let souvenirs: Vec<String> = needs
                    .iter()
                    .filter_map(|need| match need {
                        Self::Souvenir(id) => Some(souvenir_name(id).to_owned()),
                        _ => None,
                    })
                    .collect();
                let mut said = Vec::new();
                if !souvenirs.is_empty() {
                    said.push(format!("{} from Formiga Hill", or_list(&souvenirs)));
                }
                said.extend(
                    needs
                        .iter()
                        .filter(|need| !matches!(need, Self::Souvenir(_)))
                        .map(|need| need.line(facts)),
                );
                lower_after_first(said).join(", or ")
            }
            _ if self.counts() && !self.met(facts) => {
                let (have, want) = self.progress(facts);
                format!("{}: {have} of {want}", self.label())
            }
            _ => self.label(),
        }
    }

    /// Whether it is counted out, so how far along it is is worth showing.
    pub fn counts(&self) -> bool {
        match self {
            Self::Counted(..) | Self::Days(_) | Self::Keepsakes(_) | Self::BattleKeepsakes(_) => {
                true
            }
            Self::AnyOf(needs) => needs.iter().any(Need::counts),
            Self::Souvenir(_) | Self::Has(_) => false,
        }
    }

    /// Whether Formiga Hill's souvenir `id` would do for it.
    pub fn takes_souvenir(&self, id: &str) -> bool {
        match self {
            Self::Souvenir(wanted) => *wanted == id,
            Self::AnyOf(needs) => needs.iter().any(|need| need.takes_souvenir(id)),
            _ => false,
        }
    }
}

/// "the daisy", "the daisy or the penny", "the daisy, the penny or the acorn".
fn or_list(things: &[String]) -> String {
    match things {
        [] => String::new(),
        [one] => one.clone(),
        [first, rest @ .., last] => {
            let mut said = vec![first.clone()];
            said.extend(rest.iter().map(|thing| thing.to_lowercase()));
            format!("{} or {}", said.join(", "), last.to_lowercase())
        }
    }
}

/// Each of a list of things said after the first begins in lower case, as it goes on a
/// sentence.
fn lower_after_first(said: Vec<String>) -> Vec<String> {
    said.into_iter()
        .enumerate()
        .map(|(at, line)| {
            if at == 0 {
                return line;
            }
            let mut letters = line.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_lowercase().chain(letters).collect()
            })
        })
        .collect()
}

/// Where a project stands for a household.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Not on the board yet: the household does not have what it is for.
    Hidden,
    /// On the board, working towards a stage, counting from 0.
    Underway(usize),
    /// A stage is ready to finish.
    Ready(usize),
    /// Every stage is finished.
    Finished,
}

impl Project {
    pub fn catalog_id(&self) -> CatalogId {
        CatalogId::known(self.id)
    }

    /// Its name, as a sentence goes on: "the window garden", "the hearth".
    pub fn the(&self) -> String {
        let name = self.name.to_lowercase();
        if name.starts_with("the ") {
            name
        } else {
            format!("the {name}")
        }
    }

    /// The household's record of it, if the board has shown it.
    pub fn record<'a>(&self, home: &'a HouseholdHome) -> Option<&'a Record> {
        home.projects
            .iter()
            .find(|record| record.project.as_str() == self.id)
    }

    /// How many of its stages the household has finished.
    pub fn done(&self, home: &HouseholdHome) -> usize {
        self.record(home)
            .map_or(0, |record| record.stages.len().min(self.stages.len()))
    }

    pub fn standing(&self, facts: &Facts) -> Standing {
        let done = self.done(facts.home);
        match self.stages.get(done) {
            None => Standing::Finished,
            Some(_) if done == 0 && !self.shows.iter().all(|need| need.met(facts)) => {
                Standing::Hidden
            }
            Some(stage) if stage.needs.iter().all(|need| need.met(facts)) => Standing::Ready(done),
            Some(_) => Standing::Underway(done),
        }
    }

    /// Whether the board has never shown it to the owner.
    pub fn is_new(&self, facts: &Facts) -> bool {
        self.record(facts.home).is_none() && self.standing(facts) != Standing::Hidden
    }

    /// Every piece and finish its stages give, with the stage that gives it, counting from 1.
    #[cfg(test)]
    pub fn gifts(&self) -> impl Iterator<Item = (usize, &'static Gift)> + '_ {
        self.stages
            .iter()
            .enumerate()
            .flat_map(|(index, stage)| stage.gifts.iter().map(move |gift| (index + 1, gift)))
    }
}

/// The board has shown `project` to the owner: it is not new any more.
pub fn seen(home: &mut HouseholdHome, project: &Project) {
    if project.record(home).is_none() && home.projects.len() < MAX_PROJECTS {
        home.projects.push(Record {
            project: project.catalog_id(),
            stages: Vec::new(),
        });
    }
}

/// Finish the stage of `project` the household is ready for, at `at`: kept for good, and a line
/// in the journal. The stage finished, if one was ready.
pub fn finish(
    snapshot: &HomeSnapshot,
    home: &mut HouseholdHome,
    project: &'static Project,
    at: OffsetDateTime,
) -> Option<&'static Stage> {
    let Standing::Ready(index) = project.standing(&Facts { snapshot, home }) else {
        return None;
    };
    seen(home, project);
    let record = home
        .projects
        .iter_mut()
        .find(|record| record.project.as_str() == project.id)?;
    record.stages.push(StageDone { at_utc: at });
    home.note(
        at,
        HomeMoment::Project {
            project: project.catalog_id(),
            stage: (index + 1) as u8,
        },
    );
    project.stages.get(index)
}

/// What a gift did in the house, for the owner to hear.
#[derive(Clone, Debug, PartialEq)]
pub enum Given {
    /// A piece made, standing in this room.
    Placed(&'static Piece, usize),
    /// A piece made, waiting on the furniture page for want of room.
    Waiting(&'static Piece),
    /// This many pieces made into this one.
    Upgraded(&'static Piece, usize),
    /// A floor or wall to choose.
    Finish(&'static str),
}

/// Give the household a stage's gifts: what is made stands in the house, if it has room, and
/// what is improved is improved where it stands.
pub fn give(home: &mut HouseholdHome, stage: &Stage) -> Vec<Given> {
    let mut given = Vec::new();
    for gift in stage.gifts {
        match *gift {
            Gift::Piece(id) => {
                let Some(piece) = catalog::piece(&CatalogId::known(id)) else {
                    continue;
                };
                match spot_for(home, piece) {
                    Some((room, x, y, turn)) => {
                        placement::add_piece(home, room, piece, x, y, turn);
                        given.push(Given::Placed(piece, room));
                    }
                    None => given.push(Given::Waiting(piece)),
                }
            }
            Gift::Upgrade { from, to } => {
                let Some(piece) = catalog::piece(&CatalogId::known(to)) else {
                    continue;
                };
                let mut made = 0;
                for room in &mut home.rooms {
                    for placed in &mut room.pieces {
                        if from.contains(&placed.piece.as_str()) {
                            placed.piece = piece.catalog_id();
                            made += 1;
                        }
                    }
                }
                given.push(Given::Upgraded(piece, made));
            }
            Gift::Finish(id) => given.push(Given::Finish(id)),
        }
    }
    given
}

/// Somewhere a new piece can stand in the house: against a far wall if it can, and never in a
/// doorway's way or over anything but a rug. The front room first.
fn spot_for(home: &HouseholdHome, piece: &'static Piece) -> Option<(usize, u8, u8, u8)> {
    if !placement::has_room_for_more(home) {
        return None;
    }
    let house = House::of(&home.rooms);
    let mut best = None;
    for (room, layout) in home.rooms.iter().enumerate() {
        for turn in [0, 1] {
            for y in 0..layout.depth {
                for x in 0..layout.width {
                    if !placement::can_place(layout, piece, x, y, turn, None) {
                        continue;
                    }
                    let footprint =
                        crate::arrange::footprint_at(&house, room as u8, piece, x, y, turn);
                    if crate::arrange::blocks_a_doorway(&house, footprint) {
                        continue;
                    }
                    // Against the back wall it faces out from is best, then any far wall, then
                    // nearest the back.
                    let against = match turn {
                        0 => y == 0,
                        _ => x == 0,
                    };
                    let score = (
                        !against,
                        x != 0 && y != 0,
                        u16::from(x) + u16::from(y),
                        room,
                    );
                    if best.as_ref().is_none_or(|(best, _)| score < *best) {
                        best = Some((score, (room, x, y, turn)));
                    }
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, spot)| spot)
}

/// Formiga Hill's souvenirs the colony has that this household has not heard of yet, newest
/// first, for saying how they count. They come home one at a time, so the newest are the ones
/// not heard of.
pub fn fresh_souvenirs<'a>(snapshot: &'a HomeSnapshot, home: &HouseholdHome) -> Vec<&'a str> {
    let mut souvenirs: Vec<_> = snapshot
        .inventory
        .iter()
        .filter_map(|item| match &item.source {
            DisplaySource::HillSouvenir { id } => Some((item.found_at_utc, id.as_str())),
            _ => None,
        })
        .collect();
    souvenirs.sort_by(|a, b| b.cmp(a));
    let fresh = souvenirs.len().saturating_sub(heard(home) as usize);
    souvenirs
        .into_iter()
        .take(fresh)
        .map(|(_, id)| id)
        .collect()
}

fn heard(home: &HouseholdHome) -> u32 {
    home.tallies
        .iter()
        .find(|tally| tally.what.as_str() == HILL_HEARD)
        .map_or(0, |tally| tally.count)
}

/// The household has now heard of every souvenir the colony has.
pub fn heard_of_souvenirs(snapshot: &HomeSnapshot, home: &mut HouseholdHome) {
    let all = snapshot
        .inventory
        .iter()
        .filter(|item| matches!(item.source, DisplaySource::HillSouvenir { .. }))
        .count() as u32;
    let heard = heard(home);
    if all > heard {
        bump(home, HILL_HEARD, all - heard);
    }
}

/// How many of the colony's souvenirs from Formiga Hill the household has heard of.
const HILL_HEARD: &str = "heard.hill";

/// The projects a fresh souvenir counts towards, among those the household is still working on.
pub fn counted_by(souvenir: &str, facts: &Facts) -> Vec<&'static Project> {
    PROJECTS
        .iter()
        .filter(|project| {
            let done = project.done(facts.home);
            project.stages[done.min(project.stages.len())..]
                .iter()
                .any(|stage| stage.needs.iter().any(|need| need.takes_souvenir(souvenir)))
                && project.standing(facts) != Standing::Hidden
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Arrival, Arrived, FLOORS, PIECES, WALLS};
    use crate::starter;
    use formiga_home_contract::limits::{MAX_PLACED_PER_HOUSEHOLD, MAX_STAGES};
    use formiga_home_contract::{DisplayId, HomeDocument, HomeState, PlacedDisplay, Spot, sample};
    use time::macros::datetime;

    const AT: OffsetDateTime = datetime!(2026-10-15 12:00 UTC);

    fn home() -> (HomeSnapshot, HouseholdHome) {
        let snapshot = sample::snapshot();
        let home = starter::home(&snapshot);
        (snapshot, home)
    }

    fn standing(project: &Project, snapshot: &HomeSnapshot, home: &HouseholdHome) -> Standing {
        project.standing(&Facts { snapshot, home })
    }

    #[test]
    fn the_projects_are_of_every_kind_and_give_only_what_the_catalogue_has_for_them() {
        assert!((3..=6).contains(&PROJECTS.len()));
        assert!(PROJECTS.len() <= MAX_PROJECTS);
        assert!(
            Count::ALL.len() < MAX_TALLIES,
            "every count fits, and what is heard"
        );
        for kind in [Kind::Construction, Kind::Improvement, Kind::Legacy] {
            assert!(
                PROJECTS.iter().any(|project| project.kind == kind),
                "{kind:?}"
            );
        }
        for project in &PROJECTS {
            assert!(CatalogId::parse(project.id).is_some(), "{}", project.id);
            assert!(
                (1..=MAX_STAGES).contains(&project.stages.len()),
                "{}",
                project.id
            );
            for (stage, gift) in project.gifts() {
                let made = Arrival::Made(project.id, stage);
                match *gift {
                    Gift::Piece(id) | Gift::Upgrade { to: id, .. } => {
                        let piece = PIECES.iter().find(|piece| piece.id == id).unwrap();
                        assert_eq!(piece.arrives, made, "{id}");
                        assert_eq!(piece.set, catalog::Set::Made, "{id}");
                    }
                    Gift::Finish(id) => {
                        let finish = catalog::finish(id).unwrap();
                        assert_eq!(finish.arrives, made, "{id}");
                    }
                }
                // An improvement is made in place: on the same footprint, keeping every place
                // something can be shown.
                if let Gift::Upgrade { from, to } = *gift {
                    let to = catalog::piece(&CatalogId::known(to)).unwrap();
                    for from in from {
                        let from = catalog::piece(&CatalogId::known(from)).unwrap();
                        assert_eq!(from.size, to.size, "{} into {}", from.id, to.id);
                        assert!(
                            to.surfaces.starts_with(from.surfaces),
                            "{} into {}",
                            from.id,
                            to.id
                        );
                    }
                }
            }
        }
        // Everything that is made is made by some project's stage.
        let made_by_a_project = |arrives: Arrival| match arrives {
            Arrival::Made(id, stage) => project(id).is_some_and(|project| {
                project.gifts().any(|(at, gift)| {
                    at == stage
                        && match *gift {
                            Gift::Piece(id) | Gift::Upgrade { to: id, .. } | Gift::Finish(id) => {
                                PIECES
                                    .iter()
                                    .any(|piece| piece.id == id && piece.arrives == arrives)
                                    || FLOORS
                                        .iter()
                                        .chain(&WALLS)
                                        .any(|finish| finish.id == id && finish.arrives == arrives)
                            }
                        }
                })
            }),
            _ => true,
        };
        for arrives in PIECES
            .iter()
            .map(|piece| piece.arrives)
            .chain(FLOORS.iter().chain(&WALLS).map(|finish| finish.arrives))
        {
            assert!(made_by_a_project(arrives), "{arrives:?}");
        }
    }

    #[test]
    fn a_stage_is_ready_once_the_household_has_lived_it_and_is_finished_only_once() {
        let (mut snapshot, mut home) = home();
        let hearth = project("project.hearth").unwrap();
        snapshot.days_lived = 3;
        assert_eq!(standing(hearth, &snapshot, &home), Standing::Underway(0));
        snapshot.days_lived = 8;
        assert_eq!(
            standing(hearth, &snapshot, &home),
            Standing::Underway(0),
            "a keepsake is wanted too"
        );
        assert!(finish(&snapshot, &mut home, hearth, AT).is_none());
        crate::keepsakes::make(
            &mut home,
            formiga_home_contract::MementoKind::Pebble,
            None,
            Vec::new(),
            AT,
        )
        .unwrap();
        assert_eq!(standing(hearth, &snapshot, &home), Standing::Ready(0));
        let stage = finish(&snapshot, &mut home, hearth, AT).unwrap();
        assert_eq!(stage.name, "Lay the hearth");
        assert_eq!(hearth.done(&home), 1);
        assert_eq!(
            home.journal.last().unwrap().moment,
            HomeMoment::Project {
                project: hearth.catalog_id(),
                stage: 1
            }
        );
        assert!(
            finish(&snapshot, &mut home, hearth, AT).is_none(),
            "the next stage asks for more"
        );
        assert_eq!(hearth.done(&home), 1);
    }

    #[test]
    fn a_souvenir_anyone_brought_home_counts_for_every_household_but_each_makes_its_own() {
        let snapshot = sample::snapshot();
        assert!(has_souvenir(&snapshot, "pressed_daisy"));
        let mut ours = starter::home(&snapshot);
        let mut theirs = HouseholdHome {
            keeper: snapshot.village[1].keeper,
            ..starter::home(&snapshot)
        };
        let garden = project("project.window_garden").unwrap();
        for _ in 0..4 {
            count(&mut ours, Count::Tending);
        }
        assert_eq!(standing(garden, &snapshot, &ours), Standing::Ready(0));
        assert_eq!(
            standing(garden, &snapshot, &theirs),
            Standing::Underway(0),
            "the daisy counts there too, but the tending is ours"
        );
        finish(&snapshot, &mut ours, garden, AT).unwrap();
        for _ in 0..4 {
            count(&mut theirs, Count::Tending);
        }
        assert_eq!(garden.done(&theirs), 0);
        assert_eq!(standing(garden, &snapshot, &theirs), Standing::Ready(0));
        let mut without = snapshot.clone();
        without.inventory.retain(|item| {
            !matches!(&item.source, DisplaySource::HillSouvenir { id } if id == "pressed_daisy")
        });
        assert_eq!(standing(garden, &without, &theirs), Standing::Underway(0));
    }

    #[test]
    fn something_made_stands_in_the_house_or_waits_for_room() {
        let (mut snapshot, mut home) = home();
        snapshot.days_lived = 8;
        crate::keepsakes::make(
            &mut home,
            formiga_home_contract::MementoKind::Pebble,
            None,
            Vec::new(),
            AT,
        )
        .unwrap();
        let hearth = project("project.hearth").unwrap();
        let mut full = home.clone();
        let stage = finish(&snapshot, &mut home, hearth, AT).unwrap();
        let given = give(&mut home, stage);
        assert!(matches!(given[0], Given::Placed(piece, 0) if piece.id == "hearth"));
        assert_eq!(given[1], Given::Finish("floor.hearthstone"));
        let placed = home.rooms[0]
            .pieces
            .iter()
            .find(|placed| placed.piece.as_str() == "hearth")
            .unwrap();
        let house = House::of(&home.rooms);
        let piece = catalog::piece(&placed.piece).unwrap();
        let footprint =
            crate::arrange::footprint_at(&house, 0, piece, placed.x, placed.y, placed.turn);
        assert!(!crate::arrange::blocks_a_doorway(&house, footprint));
        let mut state = HomeState::new(&snapshot.colony_key);
        state.set_household(home.clone());
        state.validate().unwrap();

        // A house with no room for more keeps it on the furniture page.
        while full.placed() < MAX_PLACED_PER_HOUSEHOLD {
            let item = DisplayId::memento(full.keeper, full.placed() as u16);
            full.rooms[0].displays.push(PlacedDisplay {
                item,
                spot: Spot::Floor { x: 0, y: 0 },
            });
        }
        let stage = finish(&snapshot, &mut full, hearth, AT).unwrap();
        assert!(matches!(give(&mut full, stage)[0], Given::Waiting(piece) if piece.id == "hearth"));
        assert!(
            catalog::piece(&CatalogId::known("hearth"))
                .unwrap()
                .available(&Arrived::of(&snapshot, &full))
        );
    }

    #[test]
    fn an_improvement_is_made_where_the_piece_stands_with_whatever_is_shown_on_it() {
        let (snapshot, mut home) = home();
        let table_project = project("project.family_table").unwrap();
        assert_eq!(
            standing(table_project, &snapshot, &home),
            Standing::Hidden,
            "not without a table"
        );
        let low_table = catalog::piece(&CatalogId::known("low_table")).unwrap();
        let (room, x, y, turn) = spot_for(&home, low_table).unwrap();
        let uid = placement::add_piece(&mut home, room, low_table, x, y, turn);
        let find = snapshot
            .inventory
            .iter()
            .find(|item| matches!(item.source, DisplaySource::DesktopFind { .. }))
            .unwrap()
            .id
            .clone();
        home.rooms[room].displays.push(PlacedDisplay {
            item: find.clone(),
            spot: Spot::On {
                piece: uid,
                slot: 1,
            },
        });
        assert_eq!(
            standing(table_project, &snapshot, &home),
            Standing::Underway(0)
        );
        for _ in 0..5 {
            count(&mut home, Count::Snacks);
        }
        let stage = finish(&snapshot, &mut home, table_project, AT).unwrap();
        let given = give(&mut home, stage);
        assert!(matches!(given[0], Given::Upgraded(piece, 1) if piece.id == "gingham_table"));
        let placed = home.rooms[room].piece(uid).unwrap();
        assert_eq!(placed.piece.as_str(), "gingham_table");
        let mut state = HomeState::new(&snapshot.colony_key);
        state.set_household(home);
        let settled = state.settled_for(&snapshot);
        assert!(
            settled
                .household(snapshot.household.keeper)
                .unwrap()
                .shown()
                .any(|item| *item == find),
            "what was on the table is on it still"
        );
    }

    #[test]
    fn a_project_is_on_the_board_only_once_the_household_has_what_it_is_for() {
        let (snapshot, mut home) = home();
        let beds = project("project.cosy_beds").unwrap();
        let had_a_bed = standing(beds, &snapshot, &home);
        assert_ne!(had_a_bed, Standing::Hidden);
        for room in &mut home.rooms {
            room.pieces
                .retain(|placed| !BEDS.contains(&placed.piece.as_str()));
        }
        assert_eq!(standing(beds, &snapshot, &home), Standing::Hidden);
        assert!(!beds.is_new(&Facts {
            snapshot: &snapshot,
            home: &home
        }));
        // Once it has been begun, it stays on the board.
        home.projects.push(Record {
            project: beds.catalog_id(),
            stages: vec![StageDone { at_utc: AT }],
        });
        assert_eq!(standing(beds, &snapshot, &home), Standing::Underway(1));
    }

    #[test]
    fn a_project_is_new_until_the_board_has_shown_it() {
        let (snapshot, mut home) = home();
        let garden = project("project.window_garden").unwrap();
        assert!(garden.is_new(&Facts {
            snapshot: &snapshot,
            home: &home
        }));
        seen(&mut home, garden);
        seen(&mut home, garden);
        assert_eq!(home.projects.len(), 1);
        assert!(!garden.is_new(&Facts {
            snapshot: &snapshot,
            home: &home
        }));
    }

    #[test]
    fn souvenirs_from_formiga_hill_are_news_once() {
        let (snapshot, mut home) = home();
        let fresh = fresh_souvenirs(&snapshot, &home);
        assert_eq!(fresh.len(), sample::SOUVENIRS.len());
        assert_eq!(
            fresh[0],
            sample::SOUVENIRS.last().unwrap().id(),
            "the newest first"
        );
        assert!(
            counted_by(
                "pressed_daisy",
                &Facts {
                    snapshot: &snapshot,
                    home: &home
                }
            )
            .iter()
            .any(|project| project.id == "project.window_garden")
        );
        heard_of_souvenirs(&snapshot, &mut home);
        assert!(fresh_souvenirs(&snapshot, &home).is_empty());
    }

    /// Whether a need can be met by living at home alone: no souvenir from Formiga Hill, no
    /// friend from another house, nothing from Formiga Battle.
    fn lived_at_home(need: &Need) -> bool {
        match need {
            Need::Souvenir(_) | Need::BattleKeepsakes(_) => false,
            Need::Counted(Count::Visits | Count::Sleepovers, _) => false,
            Need::AnyOf(needs) => needs.iter().any(lived_at_home),
            _ => true,
        }
    }

    #[test]
    fn no_stage_waits_on_formiga_hill_or_on_friends_from_other_houses() {
        for project in &PROJECTS {
            // What is for Formiga Battle's cups is on the board only once one has come home.
            if project
                .shows
                .iter()
                .any(|need| matches!(need, Need::BattleKeepsakes(_)))
            {
                continue;
            }
            for stage in project.stages {
                for need in stage.needs {
                    assert!(lived_at_home(need), "{}: {need:?}", stage.name);
                }
            }
        }
    }

    #[test]
    fn every_souvenir_from_formiga_hill_counts_towards_something_and_so_do_battle_keepsakes() {
        for souvenir in formiga_core::Souvenir::ALL {
            assert!(
                PROJECTS
                    .iter()
                    .any(|project| project.stages.iter().any(|stage| stage
                        .needs
                        .iter()
                        .any(|need| need.takes_souvenir(souvenir.id())))),
                "{souvenir:?} counts for nothing"
            );
        }
        let (mut snapshot, home) = home();
        let podium = project("project.podium").unwrap();
        snapshot
            .inventory
            .retain(|item| !matches!(item.source, DisplaySource::BattleKeepsake { .. }));
        assert_eq!(standing(podium, &snapshot, &home), Standing::Hidden);
        let cup = formiga_core::BattleKeepsake::BackyardCupTrophy.id();
        snapshot.inventory.push(formiga_home_contract::DisplayItem {
            id: DisplayId::battle_keepsake(cup).unwrap(),
            source: DisplaySource::BattleKeepsake { id: cup.to_owned() },
            name: "Backyard Cup trophy".to_owned(),
            ink: None,
            modes: vec![formiga_home_contract::DisplayMode::FallbackCard],
            found_at_utc: AT,
            finder_name: None,
        });
        assert_eq!(battle_keepsakes(&snapshot), 1);
        assert_eq!(standing(podium, &snapshot, &home), Standing::Underway(0));
    }

    #[test]
    fn a_stage_that_takes_a_souvenir_or_more_of_home_life_says_both() {
        let (mut snapshot, mut home) = home();
        snapshot.inventory.retain(|item| {
            !matches!(&item.source, DisplaySource::HillSouvenir { id } if id == "pressed_daisy")
        });
        let garden = project("project.window_garden").unwrap();
        let either = &garden.stages[0].needs[0];
        for _ in 0..3 {
            count(&mut home, Count::Tending);
        }
        let facts = Facts {
            snapshot: &snapshot,
            home: &home,
        };
        assert_eq!(
            either.line(&facts),
            "Pressed daisy from Formiga Hill, or plants tended: 3 of 10"
        );
        assert!(!either.met(&facts));
        let fair = &project("project.puppet_theatre").unwrap().stages[1].needs[0];
        assert!(
            fair.label().starts_with(
                "Fairground ticket, sack-race rosette or tug-of-war rope from Formiga Hill, or \
                 games played"
            ),
            "{}",
            fair.label()
        );
        for _ in 0..7 {
            count(&mut home, Count::Tending);
        }
        let facts = Facts {
            snapshot: &snapshot,
            home: &home,
        };
        assert!(either.met(&facts), "ten plants tended do instead");
        assert_eq!(either.line(&facts), "Plants tended");
        assert_eq!(standing(garden, &snapshot, &home), Standing::Ready(0));
    }

    #[test]
    fn every_bed_and_basket_is_made_cosier_in_its_own_set_s_look() {
        let beds = project("project.cosy_beds").unwrap();
        // The guest bedroll is rolled up, not made.
        for bed in PIECES.iter().filter(|piece| {
            piece.family == catalog::Family::Beds
                && piece.set != catalog::Set::Made
                && piece.id != "bedroll"
        }) {
            assert!(
                beds.stages[0].gifts.iter().any(
                    |gift| matches!(gift, Gift::Upgrade { from, .. } if from.contains(&bed.id))
                ),
                "{} gets no quilt",
                bed.id
            );
            assert!(BEDS.contains(&bed.id), "{}", bed.id);
        }
        let (snapshot, mut home) = home();
        for room in &mut home.rooms {
            room.pieces
                .retain(|placed| !BEDS.contains(&placed.piece.as_str()));
        }
        let moss = catalog::piece(&CatalogId::known("moss_bed")).unwrap();
        let (room, x, y, turn) = spot_for(&home, moss).unwrap();
        let uid = placement::add_piece(&mut home, room, moss, x, y, turn);
        assert_ne!(
            standing(beds, &snapshot, &home),
            Standing::Hidden,
            "a moss bed is a bed"
        );
        for _ in 0..6 {
            count(&mut home, Count::Rests);
        }
        let stage = finish(&snapshot, &mut home, beds, AT).unwrap();
        give(&mut home, stage);
        assert_eq!(
            home.rooms[room].piece(uid).unwrap().piece.as_str(),
            "moss_quilt_bed"
        );
    }
}
