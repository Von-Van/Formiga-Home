//! The layout gallery: arrangements kept by name, to use again in this house or another, and
//! passed on to someone else as a file.
//!
//! A design is only what Arrange Mode makes: each room's size, floor and walls, the pieces in it
//! and where the colony's things are shown. It never carries who lives in the house, whose house
//! it was, what anyone liked, the journal, the projects, or a keepsake, so passing one on tells
//! nobody anything about the colony it came from beyond which of Formiga's finds were on show.
//!
//! The gallery is Home's own, in its data folder, like the window's place: Desktop never sees it.
//! Using a design is an ordinary arrangement, handed back through the household contract like any
//! other, so no part of the contract changes for it.
//!
//! A design is fitted to the house it is used in before anything changes: each piece where the
//! design had it if it fits, or as near as it will; and whatever cannot come is named, with why,
//! for the owner to see first. Nothing the household has not got arrives with a design: a piece
//! comes only once it has arrived here, and a find only if the colony has it.
//!
//! Nothing in a design file is trusted. It is read only up to a size, and every room in it is
//! checked by the contract's own rules for a home before it is let into the gallery.

use crate::catalog::{self, Arrived, Finish, Piece};
use crate::house::{Height, House};
use crate::placement;
use formiga_home_contract::limits::MAX_ROOMS;
use formiga_home_contract::{
    DisplayId, HomeDocument, HomeSnapshot, HomeState, HouseholdHome, PlacedDisplay, PlacedPiece,
    RoomLayout, Spot, TravelerId, WallSide,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// What a design file's name ends with, after a dot.
pub const EXTENSION: &str = "formiga-home-design";
/// What a design file says it is.
pub const FORMAT: &str = "formiga.home.design";
/// The design format this Home writes, and the newest it reads.
pub const VERSION: u32 = 1;
/// The largest design file Home reads: three full rooms written out, with room to spare.
pub const MAX_DESIGN_BYTES: u64 = 64 * 1024;
/// How many designs the gallery keeps.
pub const MAX_DESIGNS: usize = 24;
/// How long a design's name may be.
pub const MAX_NAME_CHARS: usize = 32;

/// A room or a whole house, as arranged, by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Design {
    pub format: String,
    pub version: u32,
    pub min_reader_version: u32,
    pub name: String,
    /// Every room of a house, used room for room; or else one room, used in whichever room the
    /// owner chooses.
    #[serde(default)]
    pub whole_house: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at_utc: OffsetDateTime,
    pub rooms: Vec<RoomLayout>,
}

/// What a design is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    House,
    Room(u8),
}

/// Why a design file could not be let in, or kept, in words for the owner.
#[derive(Debug, PartialEq, Eq)]
pub struct DesignError(pub String);

impl std::fmt::Display for DesignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DesignError {}

fn refuse(why: &str) -> DesignError {
    DesignError(why.to_owned())
}

const NOT_A_DESIGN: &str = "it is not a Formiga Home design";

impl Design {
    /// The household's house, or one room of it, as a design called `name`: only the rooms and
    /// what is in them. A keepsake is the household's own and stays behind.
    pub fn of(home: &HouseholdHome, scope: Scope, name: &str, at: OffsetDateTime) -> Self {
        let mut rooms = match scope {
            Scope::House => home.rooms.clone(),
            Scope::Room(room) => home
                .rooms
                .get(usize::from(room))
                .or(home.rooms.first())
                .cloned()
                .into_iter()
                .collect(),
        };
        for room in &mut rooms {
            room.displays
                .retain(|shown| shown.item.source() != DisplayId::MEMENTO);
            if scope != Scope::House {
                room.plan = None;
            }
        }
        let name = formiga_travel::sanitize_text(name, MAX_NAME_CHARS);
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            min_reader_version: 1,
            name: if name.is_empty() {
                "A design".to_owned()
            } else {
                name
            },
            whole_house: scope == Scope::House,
            saved_at_utc: at.replace_nanosecond(0).unwrap_or(at),
            rooms,
        }
    }

    /// Whether it holds together: a header this Home reads, a name in plain words, and rooms
    /// that make a home by the contract's own rules.
    pub fn validate(&self) -> Result<(), DesignError> {
        if self.format != FORMAT || self.min_reader_version == 0 {
            return Err(refuse(NOT_A_DESIGN));
        }
        if self.min_reader_version > VERSION || self.version < self.min_reader_version {
            return Err(refuse(
                "it was made by a newer Formiga Home: update Home to open it",
            ));
        }
        if !formiga_travel::is_sanitized(&self.name, MAX_NAME_CHARS) {
            return Err(refuse("its name is not plain words"));
        }
        let rooms = if self.whole_house { MAX_ROOMS } else { 1 };
        if self.rooms.is_empty() || self.rooms.len() > rooms {
            return Err(refuse("it has more rooms than a house can"));
        }
        // Checked as a home by the contract: no keepsake can be in it, since a keepsake is only
        // ever shown in the house it is from, and this house has none.
        let mut state = HomeState::new("0000000000000000");
        state.set_household(HouseholdHome {
            keeper: TravelerId(0),
            rooms: self.rooms.clone(),
            likings: Vec::new(),
            mementos: Vec::new(),
            journal: Vec::new(),
            stays_out: Vec::new(),
            projects: Vec::new(),
            tallies: Vec::new(),
        });
        state
            .validate()
            .map_err(|error| DesignError(format!("its rooms do not add up: {error}")))
    }
}

/// A design as the text of a design file.
pub fn encode(design: &Design) -> Result<Vec<u8>, DesignError> {
    design.validate()?;
    let mut bytes =
        serde_json::to_vec_pretty(design).map_err(|error| DesignError(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// The design in a design file's text, checked before anything else sees it.
pub fn decode(bytes: &[u8]) -> Result<Design, DesignError> {
    if bytes.len() as u64 > MAX_DESIGN_BYTES {
        return Err(refuse("it is larger than Home takes"));
    }
    // The header first, so a newer design is told apart from something that is not one.
    #[derive(Deserialize)]
    struct Header {
        format: String,
        min_reader_version: u32,
    }
    let header: Header = serde_json::from_slice(bytes).map_err(|_| refuse(NOT_A_DESIGN))?;
    if header.format != FORMAT {
        return Err(refuse(NOT_A_DESIGN));
    }
    if header.min_reader_version > VERSION {
        return Err(refuse(
            "it was made by a newer Formiga Home: update Home to open it",
        ));
    }
    let design: Design = serde_json::from_slice(bytes).map_err(|_| refuse(NOT_A_DESIGN))?;
    design.validate()?;
    Ok(design)
}

/// Reads a design file, refusing one too large before reading any of it.
pub fn read(path: &Path) -> Result<Design, DesignError> {
    let metadata = std::fs::metadata(path).map_err(|error| DesignError(error.to_string()))?;
    if !metadata.is_file() {
        return Err(refuse(NOT_A_DESIGN));
    }
    if metadata.len() > MAX_DESIGN_BYTES {
        return Err(refuse("it is larger than Home takes"));
    }
    let bytes = formiga_home_contract::read_bounded(path, MAX_DESIGN_BYTES)
        .map_err(|error| DesignError(error.to_string()))?;
    decode(&bytes)
}

/// Writes a design file at `to`, whole or not at all.
pub fn export(design: &Design, to: &Path) -> Result<(), DesignError> {
    let bytes = encode(design)?;
    formiga_travel::write_atomically(to, &bytes).map_err(|error| DesignError(error.to_string()))
}

/// A file name for a design: its name in plain letters, and the design file's ending.
pub fn file_name(design: &Design) -> String {
    let mut stem: String = design
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if stem.is_empty() {
        stem = "Design".to_owned();
    }
    format!("{stem}.{EXTENSION}")
}

/// One design in the gallery.
#[derive(Clone, Debug, PartialEq)]
pub struct Kept {
    pub design: Design,
    /// Opened from a file, rather than saved here.
    pub opened: bool,
    file: Option<PathBuf>,
}

/// Every design kept, in the order they were kept.
pub struct Gallery {
    folder: Option<PathBuf>,
    pub kept: Vec<Kept>,
}

impl Gallery {
    /// The gallery in Home's data folder `data`, or one kept only while the window is open. A
    /// file in it that does not read is left where it is, and passed over.
    pub fn open(data: Option<&Path>) -> Self {
        let folder = data.map(|data| data.join("designs"));
        let mut kept = Vec::new();
        if let Some(entries) = folder
            .as_deref()
            .and_then(|dir| std::fs::read_dir(dir).ok())
        {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect();
            files.sort();
            for path in files.into_iter().take(MAX_DESIGNS * 2) {
                match read(&path) {
                    Ok(design) => kept.push(Kept {
                        design,
                        opened: path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with("opened-")),
                        file: Some(path),
                    }),
                    Err(error) => {
                        eprintln!("formiga-home: passing over {}: {error}", path.display());
                    }
                }
            }
        }
        kept.truncate(MAX_DESIGNS);
        Self { folder, kept }
    }

    pub fn is_full(&self) -> bool {
        self.kept.len() >= MAX_DESIGNS
    }

    /// Keep a design, saved here or `opened` from a file. Where it is in the gallery.
    pub fn keep(&mut self, design: Design, opened: bool) -> Result<usize, DesignError> {
        design.validate()?;
        if self.is_full() {
            return Err(refuse(
                "the gallery is full: remove a design to make room for another",
            ));
        }
        let file = match &self.folder {
            Some(folder) => {
                std::fs::create_dir_all(folder).map_err(|error| DesignError(error.to_string()))?;
                let stamp = OffsetDateTime::now_utc().unix_timestamp();
                let prefix = if opened { "opened" } else { "saved" };
                let path = (0..)
                    .map(|n| folder.join(format!("{prefix}-{stamp}-{n}.json")))
                    .find(|path| !path.exists())
                    .expect("one of the names is free");
                export(&design, &path)?;
                Some(path)
            }
            None => None,
        };
        self.kept.push(Kept {
            design,
            opened,
            file,
        });
        Ok(self.kept.len() - 1)
    }

    /// Remove a design from the gallery, and its file with it. Nothing else changes: a house
    /// arranged with it stays as it is.
    pub fn remove(&mut self, index: usize) -> Result<(), DesignError> {
        let Some(kept) = self.kept.get(index) else {
            return Ok(());
        };
        if let Some(file) = &kept.file {
            match std::fs::remove_file(file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(DesignError(error.to_string())),
            }
        }
        self.kept.remove(index);
        Ok(())
    }
}

/// Why something in a design does not come with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Why {
    /// This Home does not know it: a newer one made the design.
    Unknown,
    /// It has not arrived for this household yet.
    NotYet,
    /// The colony does not have it.
    NotHad,
    /// There is nowhere in the room for it.
    NoSpace,
    /// The house has as much in it as a house can.
    Full,
}

/// What does not come with a design.
#[derive(Clone, Debug, PartialEq)]
pub enum Omitted {
    /// A room of the design, for which the house has no room.
    Room,
    Floor(Option<&'static Finish>, Why),
    Wall(Option<&'static Finish>, Why),
    Piece(Option<&'static Piece>, Why),
    Thing(DisplayId, Why),
}

/// A design as it would be in one household's house: the home it would make, and everything
/// the owner should know before it is used.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub home: HouseholdHome,
    /// The rooms of the house it would change.
    pub rooms: Vec<u8>,
    pub omitted: Vec<Omitted>,
    /// Shown in those rooms now and not in the design: back in the drawer.
    pub put_away: Vec<DisplayId>,
    /// Shown in another house now, and moved here.
    pub moved: Vec<(DisplayId, TravelerId)>,
}

/// The design fitted to the house `keeper` keeps, as `state` has it: a whole house room for
/// room, or one room into the house's room `room`. Everything else about the household is left
/// as it is: its likings, keepsakes, journal and projects, its walls and its doorways. A piece
/// already there that the design has too keeps its name, so what anyone thought of it stays.
pub fn fit(
    design: &Design,
    state: &HomeState,
    keeper: TravelerId,
    snapshot: &HomeSnapshot,
    room: u8,
) -> Option<Fitted> {
    let before = state.household(keeper)?;
    let now = Arrived::of(snapshot, before);
    let house = House::of(&before.rooms);
    let targets: Vec<Option<usize>> = if design.whole_house {
        (0..design.rooms.len())
            .map(|index| (index < before.rooms.len()).then_some(index))
            .collect()
    } else {
        vec![Some(usize::from(room).min(before.rooms.len() - 1))]
    };
    let mut home = before.clone();
    let mut omitted = Vec::new();
    let mut fresh = placement::next_uid(before);
    let mut shown_before = Vec::new();
    for target in targets.iter().flatten() {
        let layout = &mut home.rooms[*target];
        shown_before.extend(layout.displays.drain(..).map(|shown| shown.item));
        layout.pieces.clear();
    }
    for (planned, target) in design.rooms.iter().zip(&targets) {
        let Some(target) = *target else {
            omitted.push(Omitted::Room);
            continue;
        };
        let names = furnish(
            &mut home,
            &before.rooms[target],
            target,
            planned,
            (&house, &now),
            &mut fresh,
            &mut omitted,
        );
        show_things(
            &mut home,
            target,
            planned,
            &names,
            (&house, snapshot),
            &mut omitted,
        );
    }
    crate::arrange::settle(&mut home);
    let moved = home
        .shown()
        .filter_map(|item| {
            state
                .shown_by(item)
                .filter(|other| *other != keeper)
                .map(|other| (item.clone(), other))
        })
        .collect();
    let put_away = shown_before
        .into_iter()
        .filter(|item| !home.shown().any(|shown| shown == item))
        .collect();
    Some(Fitted {
        home,
        rooms: targets
            .into_iter()
            .flatten()
            .map(|room| room as u8)
            .collect(),
        omitted,
        put_away,
        moved,
    })
}

/// The room's finishes and pieces from the design, as far as they have arrived and fit. Which
/// name in the house each of the design's pieces came in under.
fn furnish(
    home: &mut HouseholdHome,
    was: &RoomLayout,
    target: usize,
    planned: &RoomLayout,
    (house, now): (&House, &Arrived),
    fresh: &mut u16,
    omitted: &mut Vec<Omitted>,
) -> Vec<(u16, u16)> {
    let finish = |id: &str| -> Result<(), (Option<&'static Finish>, Why)> {
        match catalog::finish(id) {
            None => Err((None, Why::Unknown)),
            Some(finish) if !finish.available(now) => Err((Some(finish), Why::NotYet)),
            Some(_) => Ok(()),
        }
    };
    let layout = &mut home.rooms[target];
    match finish(planned.floor.as_str()) {
        Ok(()) => layout.floor = planned.floor.clone(),
        Err((finish, why)) => omitted.push(Omitted::Floor(finish, why)),
    }
    match finish(planned.wall.as_str()) {
        Ok(()) => layout.wall = planned.wall.clone(),
        Err((finish, why)) => omitted.push(Omitted::Wall(finish, why)),
    }
    let mut reusable: Vec<&PlacedPiece> = was.pieces.iter().collect();
    let mut names = Vec::new();
    for placed in &planned.pieces {
        let Some(piece) = catalog::piece(&placed.piece) else {
            omitted.push(Omitted::Piece(None, Why::Unknown));
            continue;
        };
        if !piece.available(now) {
            omitted.push(Omitted::Piece(Some(piece), Why::NotYet));
            continue;
        }
        if !placement::has_room_for_more(home) {
            omitted.push(Omitted::Piece(Some(piece), Why::Full));
            continue;
        }
        let layout = &home.rooms[target];
        let Some((x, y, turn)) = stand(layout, house, target as u8, piece, placed) else {
            omitted.push(Omitted::Piece(Some(piece), Why::NoSpace));
            continue;
        };
        let uid = match reusable.iter().position(|old| old.piece == placed.piece) {
            Some(index) => reusable.remove(index).uid,
            None => {
                let uid = *fresh;
                *fresh = fresh.saturating_add(1);
                uid
            }
        };
        home.rooms[target].pieces.push(PlacedPiece {
            uid,
            piece: placed.piece.clone(),
            x,
            y,
            turn,
        });
        names.push((placed.uid, uid));
    }
    names
}

/// Where a piece of the design can stand in the room: where the design has it if it fits there,
/// otherwise the nearest place it does, at its own turn if it can be.
fn stand(
    layout: &RoomLayout,
    house: &House,
    room: u8,
    piece: &Piece,
    placed: &PlacedPiece,
) -> Option<(u8, u8, u8)> {
    let turns = (0..4).map(|step| (placed.turn + step) % 4);
    turns.into_iter().find_map(|turn| {
        let (w, d) = piece.size_at(turn);
        if w > layout.width || d > layout.depth {
            return None;
        }
        let x0 = i32::from(placed.x.min(layout.width - w));
        let y0 = i32::from(placed.y.min(layout.depth - d));
        let mut places: Vec<(u8, u8)> = (0..=layout.depth - d)
            .flat_map(|y| (0..=layout.width - w).map(move |x| (x, y)))
            .collect();
        places.sort_by_key(|(x, y)| (i32::from(*x) - x0).pow(2) + (i32::from(*y) - y0).pow(2));
        places.into_iter().find_map(|(x, y)| {
            let fits = placement::can_place(layout, piece, x, y, turn, None)
                && (piece.flat
                    || !crate::arrange::blocks_a_doorway(
                        house,
                        crate::arrange::footprint_at(house, room, piece, x, y, turn),
                    ));
            fits.then_some((x, y, turn))
        })
    })
}

/// The design's things shown in the room, as far as the colony has them and there is a place
/// for them: on the piece the design had them on, on the same wall, or on the floor near where
/// it had them.
fn show_things(
    home: &mut HouseholdHome,
    target: usize,
    planned: &RoomLayout,
    names: &[(u16, u16)],
    (house, snapshot): (&House, &HomeSnapshot),
    omitted: &mut Vec<Omitted>,
) {
    for shown in &planned.displays {
        let item = snapshot
            .item(&shown.item)
            .filter(|item| item.id.source() != DisplayId::MEMENTO);
        let Some(item) = item else {
            omitted.push(Omitted::Thing(shown.item.clone(), Why::NotHad));
            continue;
        };
        let elsewhere = home.shown().any(|there| there == &item.id);
        if !elsewhere && !placement::has_room_for_more(home) {
            omitted.push(Omitted::Thing(item.id.clone(), Why::Full));
            continue;
        }
        let layout = &home.rooms[target];
        let free = |spot: Spot| placement::can_show(layout, item, spot).is_some();
        let spot = match shown.spot {
            Spot::On { piece, slot } => names
                .iter()
                .find(|(planned, _)| *planned == piece)
                .and_then(|(_, uid)| {
                    let slots = layout
                        .piece(*uid)
                        .map_or(0, |placed| placement::surfaces(placed).len() as u8);
                    std::iter::once(slot)
                        .chain(0..slots)
                        .map(|slot| Spot::On { piece: *uid, slot })
                        .find(|spot| free(*spot))
                }),
            Spot::Wall { side, at } => {
                let hangs = |side: WallSide, at: u8| {
                    house
                        .wall(target as u8, side, at)
                        .is_some_and(|wall| wall.height == Height::Full && !wall.door)
                        && free(Spot::Wall { side, at })
                };
                let other = match side {
                    WallSide::North => WallSide::West,
                    WallSide::West => WallSide::North,
                };
                [side, other].into_iter().find_map(|side| {
                    let length = layout.wall_length(side);
                    let at = i32::from(at.min(length.saturating_sub(1)));
                    let mut along: Vec<u8> = (0..length).collect();
                    along.sort_by_key(|there| (i32::from(*there) - at).abs());
                    along
                        .into_iter()
                        .find(|there| hangs(side, *there))
                        .map(|at| Spot::Wall { side, at })
                })
            }
            Spot::Floor { x, y } => {
                let (x0, y0) = (
                    i32::from(x.min(layout.width - 1)),
                    i32::from(y.min(layout.depth - 1)),
                );
                let (rx, ry) = house
                    .room(target as u8)
                    .map_or((0, 0), |room| (room.x, room.y));
                let mut tiles: Vec<(u8, u8)> = (0..layout.depth)
                    .flat_map(|y| (0..layout.width).map(move |x| (x, y)))
                    .collect();
                tiles.sort_by_key(|(x, y)| {
                    (i32::from(*x) - x0).pow(2) + (i32::from(*y) - y0).pow(2)
                });
                tiles
                    .into_iter()
                    .find(|(x, y)| {
                        free(Spot::Floor { x: *x, y: *y })
                            && !crate::arrange::blocks_a_doorway(
                                house,
                                placement::Footprint {
                                    x: rx + x,
                                    y: ry + y,
                                    w: 1,
                                    d: 1,
                                },
                            )
                    })
                    .map(|(x, y)| Spot::Floor { x, y })
            }
        };
        let Some(spot) = spot else {
            omitted.push(Omitted::Thing(item.id.clone(), Why::NoSpace));
            continue;
        };
        home.take_down(&item.id);
        home.rooms[target].displays.push(PlacedDisplay {
            item: item.id.clone(),
            spot,
        });
    }
}

#[cfg(test)]
mod tests;
