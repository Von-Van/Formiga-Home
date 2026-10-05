# The household contract

Formiga Desktop owns the living colony. Formiga Home is given one household at a time, as a
deliberate, versioned projection, and gives back only what Desktop is willing to keep. The
contract is the `formiga-home-contract` crate. Its `lib.rs` is the authority, then `snapshot.rs`,
`state.rs`, `replies.rs`, `inventory.rs` and `accept.rs`. Golden fixtures for every document are
in its `tests/fixtures`.

The crate is a draft kept beside Home. It follows the precedent of Desktop's `formiga-travel` and
is built on it: a resident is exactly the `Traveler` a trip would carry, documents are written
whole the same way, and text is made safe the same way. When Desktop adopts the crate it moves
into Desktop's workspace, as `formiga-travel` lives there for Formiga Hill, and Home takes it by
the same release tag as the other three.

## Who owns what

| Who | Owns | Never touches |
| --- | --- | --- |
| Desktop | The colony; which household lives in which house; the layouts it last accepted (`home/state.json`, beside the colony and never inside it); every session directory | Home's window and its data folder |
| Home | The room on screen, the residents' life in it, its own window and lock | The colony file, ever |
| Formiga Hill | Nothing here. A souvenir reaches Home only after Desktop has kept it | Home's documents |

## A visit, file by file

Desktop opens a house when its owner clicks it in the village. It settles whatever the household
was in the middle of, saves the colony, and writes a fresh session directory:

| File | Written by | What it is |
| --- | --- | --- |
| `snapshot.json` | Desktop | `HomeSnapshot`: the household, its residents, any visitors lent for the visit, their bonds, the village's keepers, everything the colony can show, and the owner's shared preferences |
| `state.json` | Desktop | `HomeState`: every household's layout as Desktop last accepted it |
| `ack.json` | Home | `HomeAck`, once: accepted, or refused as `UnsupportedVersion { reads }`, `Invalid` or `Busy` |
| `result-state.json` | Home | `HomeResult`: every layout, whole, as Home would have it. Written each time the owner leaves Arrange Mode, after every change, and again on leaving; the last one written is the answer |
| `receipt.json` | Home | `HomeReceipt`, once, on leaving: at most one `HomeVisit` |
| `recall.json` | Desktop | `HomeRecall`, if Desktop ends the visit first |

Home is started as `formiga-home --formiga-home <absolute session directory>` and with nothing
else. Every file is written whole to a temporary name and renamed into place.

While the session is open, the household's residents and any visitors Desktop lent are indoors:
Desktop keeps them off the desktop, and every other companion lives as usual. When Home exits, however it exits, Desktop
reads the answers, keeps what [`accept_result`] lets it keep, and lets the household out again
through its ordinary village life. It always fails toward home. If Home is missing, refuses,
crashes or writes something that does not check out, the household simply comes back out, and the
layouts Desktop last accepted stand.

### Home's side

`crates/formiga-home/src/session.rs`:

1. **Arrive.** The session id is read from the directory's name and nothing else about the path
   is trusted. The snapshot and the state are read bounded, decoded header first, and checked
   against each other: the snapshot must be for this session, and its residents drawable by this
   build's `formiga-travel`. A state kept for another colony is set aside in favour of a fresh
   one. `ack.json` is written once.
2. **Live.** Once a second, Home checks for `recall.json`, or for `snapshot.json` having gone.
   Either one ends the visit with nothing more written.
3. **Arrange.** Every change hands back the whole state as `result-state.json`, so a crash loses
   nothing that was arranged.
4. **Leave.** The result once more, then `receipt.json` with a `HomeVisit { household,
   arrived_at_utc, left_at_utc }` if the snapshot offers `visit_record`.

A Home window holds `open.lock` in its data folder, so a visit that arrives while another window
is open is refused as `Busy`.

## What Desktop keeps

`accept_result(seal, snapshot, previous, result)` is the whole rule. Desktop runs it on the result
Home wrote last.

- A result for another session, snapshot or state, or another colony, changes nothing.
- Only the visited household's home is taken from the result. Any other household's home in it is
  set aside, except that whatever the visited household now shows is taken down wherever else it
  was shown. A find moved between houses is moved, never copied.
- Nothing is kept on show that the colony does not have, or that cannot be shown where it is:
  only a thing that stands on the floor may be on the floor.
- A household whose keeper no longer keeps a house is let go, and what it showed is free again.
- A liking is kept only for someone who lives in the house, and only for something still in it.
- If what comes out does not validate, the homes stay as they were.

`HomeState::settled_for(snapshot)` applies the same tidying to the state Desktop sends, so Home
never opens on a layout the colony has outgrown.

## The snapshot

- **Residents.** The keeper first, then everyone `house_slot_for` puts in its house, in the order
  they arrived: a mini always lives with its big version. Each resident is the very `Traveler`
  `formiga_travel::project_colony` makes, so appearance, temperament, traits, habits, stature,
  pace and what it wears are Desktop's own projection. Bonds are those between residents only, in
  travel's bands.
- **Visitors**, since version 2. Desktop lends a visit the household's closest friends from other
  houses who are free. `likely_visitors` names the candidates, closest first and two at most, and
  `project_household` takes whichever Desktop lends. Each is a full-size companion who keeps a
  house of its own, projected exactly as a resident is.
- **The village.** Every house's keeper by name, so Home can say "in Biscuit's house".
- **The inventory.** Everything the colony has that a house can show:
  - every scrapbook find, as `find.<variant>`, with its colony's inks resolved by Desktop;
  - every souvenir Desktop has kept from Formiga Hill, as `souvenir.<Hill's id>`.

  Each thing lists the ways it may be shown, best first. Every one includes `fallback_card`, so
  nothing the colony has ever lacks a way onto a shelf or a wall.
- **The house's outside** (`household.style`) is a hint for how the inside first looks, never an
  owner of it.
- **Days lived**, for the few pieces of furniture that arrive with time.
- **The colony key** is a one-way digest of the colony's seed under Home's own label. It is not a
  trip's colony id, so the two apps' records cannot be matched up.

## The state

`HomeState` holds up to twelve households, each keyed by its keeper. Each has one to three rooms,
at most 96 pieces and displays in all, and, since version 2, up to 48 likings: how often each
resident has chosen a piece (by room and uid) or a find (by display id). What a resident has chosen
most of a kind is its favourite. Likings are flavour, kept by Home and read by nothing else. A
room has a size in tiles, a floor and a wall finish, furniture as `{ uid, piece, x, y, turn }`,
and displays as `{ item, spot }`. Since version 3 a room may also have:

- `plan: { x, y }`, where it stands on its house's plan: the far corner of its floor, in tiles
  from the first room's, which is always at the origin. No two rooms stand in one place, and none
  is more than 32 tiles from the first. A room with no place given is set out by Home.
- `kind`, what kind of room it is, by Home's catalogue identifier, such as `room.nook`.
- `doors: [{ side, at }]`, up to four doorways in its two far walls, `north` or `west`, `at` tiles
  along. What a doorway opens onto is whatever is on the wall's other side on the plan: another
  room of the house, or outside, where visitors come in. Nothing hangs in a doorway.

A spot is one of:

- `on { piece, slot }`, a surface of a piece in the same room;
- `wall { side, at }`, `north` or `west`;
- `floor { x, y }`.

Pieces and finishes are named by Home's catalogue identifiers. Desktop checks only that each is
written as an identifier, never what it names, so Home's catalogue can grow without Desktop. A
piece a newer Home has, and an older one does not, is kept in its place and left alone.

## Versions and bounds

Every document carries `format`, `version` and `min_reader_version`, read exactly as
`formiga-travel` reads its own. A reader accepts a document whose `min_reader_version` it reaches,
ignores fields it does not know, and refuses anything else for its version. Enums that are only
ever matched against (capabilities, display modes, sources, effects) read an unknown value as
`unknown` instead. The snapshot also names the travel version its residents are written in.
Every document is size-limited before it is parsed, every list and string is bounded, and every
string is checked as plain text: see `limits` in `lib.rs`.

| Home version | What it added |
| --- | --- |
| 1 | Everything |
| 2 | `visitors` in the snapshot; `likings` in each home. Both optional, so a version 1 reader still reads either |
| 3 | `plan`, `kind` and `doors` in each room: a house of rooms, and its doorways. All optional, so an older reader still reads a house, as a set of rooms |

## Finding Home

`discovery` in `lib.rs` names what Desktop looks for, the same arrangement as Formiga Hill's:

- On macOS, the bundle id `com.formiga.home`, with `FormigaHomeVersion` in its `Info.plist`.
- On Windows, `HKCU\Software\Formiga\Home` (or the same key under `HKLM`), with `Path`, `Version`
  and `HomeVersion` values.
- For development, `FORMIGA_HOME_PATH`.

`formiga-home --home-version` prints the version for packaging scripts to write.
