//! What Home keeps of its own, in its own data folder: where its window was, a lock that says a
//! window is open, and, for rehearsals only, the homes it would otherwise have handed back to
//! Desktop. A visit from Desktop keeps nothing here: the homes are Desktop's to keep.

use formiga_home_contract::{HomeDocument, HomeState, read_document, write_document};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

/// Home's data folder: `FORMIGA_HOME_DATA_DIR` if it is set, for development and tests.
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("FORMIGA_HOME_DATA_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("com", "Formiga", "Formiga Home")
        .map(|dirs| dirs.data_dir().to_owned())
}

const LOCK: &str = "open.lock";

/// Held for as long as a Home window is open. The lock is the operating system's, so it goes with
/// the window however that closes.
pub struct Open {
    _lock: Option<File>,
}

/// Another Home window is open.
#[derive(Debug, PartialEq, Eq)]
pub struct Busy;

/// Takes the lock in `data`, or says another Home has it. Anything else that goes wrong is not
/// another Home, and nothing is refused for it.
pub fn take(data: &Path) -> Result<Open, Busy> {
    let opened = std::fs::create_dir_all(data).and_then(|()| {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(data.join(LOCK))
    });
    let Ok(file) = opened else {
        return Ok(Open { _lock: None });
    };
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => Err(Busy),
        Ok(()) | Err(TryLockError::Error(_)) => Ok(Open { _lock: Some(file) }),
    }
}

/// Where the window was and how big, in logical points, as the notebook on Desktop remembers its
/// own.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPlace {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

const WINDOW: &str = "window.json";

impl WindowPlace {
    pub fn load(data: &Path) -> Option<Self> {
        let bytes = std::fs::read(data.join(WINDOW)).ok()?;
        let place: Self = serde_json::from_slice(&bytes).ok()?;
        let sane = [place.x, place.y, place.width, place.height]
            .iter()
            .all(|value| value.is_finite())
            && (320.0..=8000.0).contains(&place.width)
            && (240.0..=8000.0).contains(&place.height);
        sane.then_some(place)
    }

    pub fn save(&self, data: &Path) {
        let written = std::fs::create_dir_all(data).and_then(|()| {
            let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
            formiga_travel::write_atomically(&data.join(WINDOW), &bytes)
        });
        if let Err(error) = written {
            eprintln!("formiga-home: could not remember where the window was: {error}");
        }
    }
}

/// The homes a rehearsal keeps for one colony, in place of Desktop.
pub struct RehearsalHomes {
    path: Option<PathBuf>,
}

impl RehearsalHomes {
    pub fn new(data: Option<&Path>, colony_key: &str) -> Self {
        Self {
            path: data.map(|data| data.join("rehearsals").join(format!("{colony_key}.json"))),
        }
    }

    /// The homes kept last time, or none. A file that cannot be read is set aside, never written
    /// over, and the rehearsal starts afresh.
    pub fn load(&self, colony_key: &str) -> HomeState {
        let Some(path) = &self.path else {
            return HomeState::new(colony_key);
        };
        match read_document::<HomeState>(path) {
            Ok((state, _)) => state,
            Err(error) if error.is_missing() => HomeState::new(colony_key),
            Err(error) => {
                let aside = path.with_extension(format!(
                    "unreadable-{}.json",
                    time::OffsetDateTime::now_utc().unix_timestamp()
                ));
                eprintln!("formiga-home: setting aside {}: {error}", path.display());
                let _ = std::fs::rename(path, aside);
                HomeState::new(colony_key)
            }
        }
    }

    /// For review: the household's house in this rehearsal grown to `rooms` rooms, as if its
    /// owner had built on, a reading nook first and then a gallery.
    pub fn grow(&mut self, snapshot: &formiga_home_contract::HomeSnapshot, rooms: usize) {
        let mut state = self.load(&snapshot.colony_key);
        let home = crate::arrange::ensure_home(&mut state, snapshot);
        if home.rooms.len() < rooms {
            state.set_household(crate::staging::grown(home, rooms, snapshot));
            if let Err(error) = self.save(&state) {
                eprintln!("formiga-home: {error:#}");
            }
        }
    }

    pub fn save(&self, state: &HomeState) -> anyhow::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        state.validate()?;
        write_document(path, state)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("formiga-home-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_second_window_finds_the_first_open_until_it_closes() {
        let data = scratch("lock");
        let first = take(&data).expect("nobody else is open");
        assert_eq!(take(&data).err(), Some(Busy));
        drop(first);
        assert!(take(&data).is_ok(), "the lock outlived its window");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_window_place_round_trips_and_nonsense_is_forgotten() {
        let data = scratch("window");
        let place = WindowPlace {
            x: 40.0,
            y: 60.0,
            width: 840.0,
            height: 620.0,
        };
        place.save(&data);
        assert_eq!(WindowPlace::load(&data), Some(place));
        std::fs::write(
            data.join(WINDOW),
            br#"{"x": 0, "y": 0, "width": 2, "height": 2}"#,
        )
        .unwrap();
        assert_eq!(WindowPlace::load(&data), None);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_unreadable_rehearsal_is_set_aside_not_written_over() {
        let data = scratch("rehearsal");
        let homes = RehearsalHomes::new(Some(&data), "0123456789abcdef");
        std::fs::create_dir_all(data.join("rehearsals")).unwrap();
        std::fs::write(data.join("rehearsals/0123456789abcdef.json"), b"{ not json").unwrap();
        let state = homes.load("0123456789abcdef");
        assert!(state.households.is_empty());
        let aside = std::fs::read_dir(data.join("rehearsals"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry.file_name().to_string_lossy().contains("unreadable"));
        assert!(aside);
        let _ = std::fs::remove_dir_all(&data);
    }
}
