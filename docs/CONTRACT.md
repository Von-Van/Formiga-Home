# The household contract

Formiga Desktop and Formiga Home are two separate apps that need to share one household without
stepping on each other. Desktop owns the living colony. Home is given one household at a time, as
a deliberate and versioned snapshot, and it gives back only what Desktop is willing to keep. This
document describes that agreement: who owns what, which files pass between the two apps during a
visit, and how Desktop decides what to keep.

The reason for being this careful is fairly practical. A colony can represent months of someone's
time, and Home is an optional extra. If Home is missing, out of date, or crashes partway through a
visit, the colony should not notice. Nearly every rule below exists to make that true.

The contract itself is the `formiga-home-contract` crate, in Formiga Desktop's workspace since
Desktop 0.67.0. Its `lib.rs` is the authority, followed by `snapshot.rs`, `state.rs`,
`replies.rs`, `inventory.rs` and `accept.rs`. Example documents for every version (the "golden
fixtures") are kept in its `tests/fixtures`. Home takes it by Desktop's release tag, with the
other Desktop crates, so their types always agree.

It follows the example of Desktop's `formiga-travel` crate, which plays the same role for Formiga
Hill, and is built on top of it: a resident is exactly the `Traveler` a trip would carry,
documents are written the same way, and text is made safe the same way.

## Who owns what

| Who | Owns | Never touches |
| --- | --- | --- |
| Desktop | The colony; which household lives in which house; the layouts it last accepted (`home/state.json`, kept beside the colony save and never inside it); every session directory | Home's window and its data folder |
| Home | The room on screen, the residents' life in it, its own window and lock | The colony save, ever |
| Formiga Hill | Nothing here. A souvenir reaches Home only after Desktop has kept it | Home's documents |

## A visit, file by file

When its owner clicks a house in the village, Desktop settles whatever the household was in the
middle of, saves the colony, and writes a fresh session directory. From then on the two apps talk
only through the files in that directory:

| File | Written by | What it is |
| --- | --- | --- |
| `snapshot.json` | Desktop | `HomeSnapshot`: the household, its residents, any visitors lent for the visit, their bonds, the village's keepers, everything the colony can show, and the owner's shared preferences |
| `state.json` | Desktop | `HomeState`: every household's home as Desktop last accepted it |
| `ack.json` | Home | `HomeAck`, written once: accepted, or refused as `UnsupportedVersion { reads }`, `Invalid` or `Busy` |
| `result-state.json` | Home | `HomeResult`: every home, whole, as Home would have it. Written each time the owner leaves Arrange Mode, after every change, and again on leaving; the last one written is the answer |
| `receipt.json` | Home | `HomeReceipt`, written once on leaving: what the visit brings home, only as far as the snapshot offers to take it |
| `recall.json` | Desktop | `HomeRecall`, if Desktop needs to end the visit first |

Desktop starts Home as `formiga-home --formiga-home <absolute session directory>`, with nothing
else. Every file is written whole under a temporary name and then renamed into place, so neither
app ever reads a half-written file.

While the session is open, the household's residents and any visitors Desktop lent are "indoors":
Desktop keeps them off the desktop, and every other companion carries on as usual. When Home
exits, however it exits, Desktop reads the answers, keeps what [`accept_result`] allows, and lets
the household back out through its ordinary village life. Everything is designed to fail toward
home. If Home is missing, refuses, crashes, or writes something that does not check out, the
household simply comes back out and the layouts Desktop last accepted still stand.

### Home's side

The steps Home takes are all in `crates/formiga-home/src/session.rs`:

1. **Arrive.** The session id is read from the directory's name, and nothing else about the path
   is trusted. The snapshot and state are read with size limits, their headers checked first,
   and then checked against each other: the snapshot must be for this session, and its residents
   must be drawable by this build's `formiga-travel`. A state kept for a different colony is set
   aside for a fresh one. `ack.json` is written once.
2. **Live.** Once a second, Home checks for `recall.json`, or for `snapshot.json` having
   disappeared. Either one ends the visit with nothing more written.
3. **Arrange.** Every change hands back the whole state as `result-state.json`, so a crash loses
   nothing that was arranged.
4. **Leave.** The result is written once more, followed by `receipt.json` (see
   [The receipt](#the-receipt)).

A Home window holds `open.lock` in its data folder, so a visit that arrives while another window
is open is refused as `Busy`.

## What Desktop keeps

`accept_result(seal, snapshot, previous, result)` is the whole rule, and Desktop runs it on the
last result Home wrote. In plain terms:

- A result for a different session, snapshot, state or colony changes nothing.
- Only the visited household's home is taken from the result. Any other household's home in it is
  set aside, with one exception: whatever the visited household now shows is taken down wherever
  else it was shown. A find moved between houses is moved, never copied.
- Nothing stays on show that the colony does not have, or that cannot be shown where it is (only
  something that stands on the floor may be on the floor).
- A household's own keepsakes are kept with its home, and stay on show only in its own house.
  None can be shown in another.
- A household whose keeper no longer keeps a house is let go, and what it showed is free again.
- A liking is kept only for someone who lives in the house, and only for something still there.
- If what comes out does not validate, the homes stay exactly as they were.

`HomeState::settled_for(snapshot)` applies the same tidying to the state Desktop sends, so Home
never opens on a layout the colony has since outgrown.

## The snapshot

The snapshot is everything Home is allowed to know about the visit.

- **Residents.** The keeper first, then everyone `house_slot_for` puts in its house, in the order
  they arrived (a mini always lives with its big version). Each resident is the very `Traveler`
  that `formiga_travel::project_colony` makes, so appearance, temperament, traits, habits,
  stature, pace and clothing all come from Desktop's own projection. Bonds are only those between
  residents, in travel's bands.
- **Visitors**, since version 2. Desktop may lend the household's closest friends from other
  houses, if they are free. `likely_visitors` names the candidates, closest first and two at most,
  and `project_household` takes whichever ones Desktop lends. Each is a full-size companion with a
  house of its own, projected exactly as a resident is.
- **The village.** Every house's keeper by name, so Home can say "in Biscuit's house".
- **The inventory.** Everything the colony has that a house can show:
  - every scrapbook find, as `find.<variant>`, with the colony's colours already resolved by
    Desktop;
  - every souvenir Desktop has kept from Formiga Hill, as `souvenir.<Hill's id>`.

  Each thing lists the ways it may be shown, best first, and every list includes
  `fallback_card`. That way nothing the colony has is ever without a way onto a shelf or a wall.
- **The house's outside** (`household.style`) is a hint for how the inside first looks, never an
  owner of it.
- **Days lived**, for the few pieces of furniture that arrive with time.
- **Capabilities**: what Desktop is willing to take back from a visit. `visit_record` offers a
  `HomeVisit`; since version 4, `bond_nudges` offers time spent together and
  `journal_moments` offers moments; since version 5, `next_door` offers to open another house
  next; and since version 6, `roommates` offers to consider a friend asked to move in. Home sends
  nothing Desktop has not offered.
- **The colony key** is a one-way digest of the colony's seed under Home's own label. It is
  intentionally different from a trip's colony id, so the two apps' records cannot be matched up.

## The state

`HomeState` holds up to twelve households, each keyed by its keeper. Each household's home has one
to three rooms, at most 96 pieces and displays in all, and, since version 2, up to 48 **likings**:
counts of how often each resident has chosen a piece (by room and uid) or a find (by display id).
Whatever a resident has chosen most of a kind is its favourite. Likings are flavour, kept by Home
and read by nothing else.

A room has a size in tiles, a floor and a wall finish, furniture as `{ uid, piece, x, y, turn }`,
and displays as `{ item, spot }`. Since version 3 a room may also have:

- `plan: { x, y }`, where it stands on the house's plan: the far corner of its floor, in tiles
  from the first room's, which is always at the origin. No two rooms share a place, and none is
  more than 32 tiles from the first. A room with no place given is set out by Home.
- `kind`, what kind of room it is, by Home's catalogue identifier, such as `room.nook`.
- `doors: [{ side, at }]`, up to four doorways in its two far walls, `north` or `west`, `at` tiles
  along. A doorway opens onto whatever is on the other side of that wall on the plan: another room,
  or outside, where visitors come in. Nothing hangs in a doorway.

A spot is one of:

- `on { piece, slot }`, a surface of a piece in the same room;
- `wall { side, at }`, `north` or `west`;
- `floor { x, y }`.

Since version 4 a home may also keep two things of its own, which Desktop stores but never makes:

- **`mementos`**: up to 24 keepsakes the household came by at home, each
  `{ serial, kind, by, of, inks, made_at_utc }`. `kind` comes from a fixed catalogue so that any
  reader can name it: a `postcard`, `jam_jar`, `pressed_flower`, `rosette` or `pebble` left by a
  visitor; a `drawing` by a little one; or a `photo` framed by the owner. `by` is who it came from,
  and `of` is who is in it, six at most. `inks` keeps each of those companions' colours as they
  were when it was made, so a photo keeps its likeness even after whoever was in it has gone home.
  A keepsake is shown as `memento.<keeper>-<serial>`, in its own house only. Desktop never sends
  one in the inventory, because Home makes them from the home itself.
- **`journal`**: up to 40 entries `{ at_utc, moment }`, oldest first, with the oldest let go when
  it is full. A moment is structured rather than written out: a friend's `visit`, a `memento`
  that came to the house, a resident's new `favourite` (a seat, a bed, a toy or a find), a
  `room` the house grew, since version 5 a friend who `stayed_over`, and since version 6 a friend
  `asked_to_move_in`. Home words these on its Journal page, and nothing else reads them.

Pieces and finishes are named by Home's catalogue identifiers. Desktop only checks that each is
written as a valid identifier, never what it names, which lets Home's catalogue grow without
Desktop having to change. A piece that a newer Home has and an older one does not is kept in place
and left alone.

## The receipt

The receipt is the only place Home can say anything about a visit beyond the layouts, so it is
kept deliberately narrow. Each effect is sent only if the snapshot offers its capability, and a
receipt holds sixteen effects at most:

- `home_visit { household, arrived_at_utc, left_at_utc }`, once, for `visit_record`, so Desktop
  can note the visit in its own words.
- `together { a, b, together, times }`, for `bond_nudges`: how often two companions spent time
  together at home, and how. The kinds are `play`, `cozy` (sat together, shared a snack, turned in
  together), `care` (a hug, comfort) and `squabble` (teasing). Each pair is named once for each
  kind, never with itself, the lesser id first, and `times` is 1 to 3. Desktop applies these by
  its own rules, and only as far as those rules allow.
- `moment { moment }`, for `journal_moments`: at most three of the visit's journal entries, the
  ones most worth a line (a keepsake first, then a friend staying over, a new room, a new
  favourite, a friend coming over), in the order they happened. Desktop words them in its own
  journal.
- `next_door { household }`, for `next_door`, at most once: the owner went over to the house
  `household` keeps. Desktop opens that house next, in a new session, if it can and will: it must
  be another house of the village the snapshot named, and `HomeReceipt::next_door` returns it only
  then. Home has already left this one; nothing else about the visit changes.
- `move_in { resident, household }`, for `roommates`, at most once: the owner asked `resident`,
  a friend lent for the visit, to come and live in the house visited, whose keeper is
  `household`, and it would like to. A request only: Desktop grants it or not by its own rules,
  which know what Home does not (who keeps which house, who would be left alone, who follows
  whom). `HomeReceipt::move_in` returns it only for a visitor and the house visited.

If there are more effects than a receipt holds, the visit, the next door, a move and the moments
come first, and time together gives way.

Home never sends prose, a change to a creature, or anything to patch into a save. One could argue
the bond nudges come close to that line. The difference is that Home only reports what happened,
in small capped counts, and Desktop alone decides whether and how much it matters.

## Versions and limits

Every document carries `format`, `version` and `min_reader_version`, read exactly as
`formiga-travel` reads its own. A reader accepts a document whose `min_reader_version` it meets,
ignores fields it does not know, and refuses anything else. Enums that are only ever matched
against (capabilities, display modes, sources, effects) read an unknown value as `unknown` instead
of failing. The snapshot also names the travel version its residents are written in.

Every document is size-limited before it is parsed, every list and string is bounded, and every
string is checked as plain text. The exact limits are in `limits` in `lib.rs`. This is less
interesting than the rest of the contract, but it is what lets either app trust a file the other
one wrote.

| Home version | What it added |
| --- | --- |
| 1 | Everything |
| 2 | `visitors` in the snapshot; `likings` in each home. Both optional, so a version 1 reader still reads either |
| 3 | `plan`, `kind` and `doors` in each room: a house of rooms, and its doorways. All optional, so an older reader still reads a house, as a set of rooms |
| 4 | `mementos` and `journal` in each home; the receipt's `together` and `moment` effects, and the `bond_nudges` and `journal_moments` capabilities that offer them. All optional, so an older reader still reads every document |
| 5 | The receipt's `next_door` effect and the `next_door` capability that offers it; a `stayed_over` moment. An older reader reads the effect as one it does not support and the moment as `unknown` |
| 6 | The receipt's `move_in` effect and the `roommates` capability that offers it; an `asked_to_move_in` moment. Read by an older reader as before |
| 7 | Desktop 0.67.0, which adopted the contract. No new fields: residents are written in travel version 4, Formiga Hill's Fairground souvenirs may be shown, and a snapshot offers only the capabilities Desktop applies |

## Finding Home

`discovery` in `lib.rs` names what Desktop looks for, following the same arrangement as Formiga
Hill:

- On macOS, the bundle id `com.formiga.home`, with `FormigaHomeVersion` in its `Info.plist`.
- On Windows, `HKCU\Software\Formiga\Home` (or the same key under `HKLM`), with `Path`, `Version`
  and `HomeVersion` values.
- For development, `FORMIGA_HOME_PATH`.

`formiga-home --home-version` prints the version for the packaging scripts to write.
