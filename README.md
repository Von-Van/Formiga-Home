# Formiga Home

Formiga Desktop is their life around you. Formiga Home is the room you are allowed to step into.
Formiga Hill is the place you go together.

Home is a separate, optional desktop app. When Formiga Desktop opens one of its village's houses,
Home shows the inside of it: a small isometric dollhouse where that household lives, drawn on the
page of a notebook like Desktop's own. You can arrange it, show off the things the colony has
found, let it grow a room or two, and spend a few minutes directing the residents more closely
than Desktop ever does. When you stop directing them, they get on with their own lives.

- **The same creatures.** Every resident is drawn by `formiga-art` from what Desktop sends, and
  behaves by its own temperament, habits, size, family and friendships.
- **Found things become physical.** Every find in the colony's scrapbook, and every souvenir
  brought home from Formiga Hill, can be set on a shelf, put in a case, stood on a table or the
  floor, or hung on a wall.
- **Friends drop by.** Desktop lends a household's closest friends from other houses for the
  visit; they knock at the front door, look round at what is on show, and go home again.
- **A house that grows.** In time a house can have a second and a third room, seen whole in one
  cutaway, with doorways between them.
- **Soft play.** No needs to keep up, no money, no decay and nothing lost by staying away.

Desktop stays the home of the colony. Home never reads the colony file and never changes a
creature. It hands Desktop back only the houses' layouts and a note that you visited, and Desktop
keeps only what checks out.

Further reading:

- [docs/DESIGN.md](docs/DESIGN.md): what Home is for, and how a house lives.
- [docs/CONTRACT.md](docs/CONTRACT.md): the household contract between Desktop and Home.

## Status

| Part | Status |
| --- | --- |
| The house of up to three rooms, its doorways and front door, seven floors and seven walls, thirty-one pieces of furniture in three sets and the house's own | Preview |
| The notebook window: the leather cover as its frame, the house on one page and short notes on the other | Preview |
| Residents living on their own, and the three-thing queue of what you ask | Preview |
| Arrange Mode, Found Things and the fallback card for anything without its own art | Preview |
| Visitors, favourites and photos | Preview |
| The household contract (`formiga-home-contract`) | Draft, kept here until Desktop adopts it |
| Opening a house from Desktop's village | Not yet in Desktop |
| Packaging for macOS and Windows | Not yet verified: the scripts assemble a bundle and an installer under Home's own names, and have not been run end to end |

Until Desktop opens houses itself, Home opens in **rehearsal**: Desktop's own sample household, or
a house in a real colony file, read-only. A rehearsal keeps its layouts in Home's data folder
exactly as Desktop would keep them, through the contract's own acceptance rules.

## A visit

The window is a notebook. Its leather cover is the frame: drag it to move the window, and use
the brass studs to close it, put it away, open it wide or take a photo. The house is drawn on the
left-hand page and notes are kept on the right; "How to…" at the foot of the notes says
everything the mouse and the keys can do.

The notebook opens on the house in **Live Mode**.

- **Choose someone.** Click a resident, or its portrait in the notes. Hover over a resident to see
  its name. The notes say a little more about whoever is chosen: what it is like, what it is
  doing, what it has come to like, and what it has been asked.
- **Ask for something.** With someone chosen, click something in the room to see what they could
  do there:
  - a seat, a deckchair or a toadstool: sit, relax or nap;
  - a bed or basket: sleep, curl up, bounce on it, or turn in with a little one;
  - a table: sit at it;
  - a rug: stretch out on it;
  - a toy, the toy box or the telescope: play, play with someone, show off;
  - the snack bowl: have a snack, or share one;
  - a lamp: switch it on or off; the fern: tend it;
  - a shelf, a case, the cabinet, the bell jar or the counter: look over everything on it;
  - anything on show: look at it, remember it, show it to someone, play with it if it is a toy,
    or try it on for a while if it is small enough to wear;
  - another resident or a visitor: say hello, sit together, play, tease, comfort, hug, as their
    temperament and bond allow.

  Click the floor to send them there, in any room; they go through the doorways. Each resident
  takes up to three things at a time. The notes show the list, and a ✕ takes one back.
- **Handle them.** Drag a resident to pick it up and put it down somewhere else, or right-click it
  for a pat. Either one lets go of everything they were asked to do.
- **Leave them be.** Whenever nobody has asked them for anything, residents choose for themselves.
- **Favourites.** Whatever a resident keeps choosing, a seat, a bed, a toy, a find, becomes its
  favourite, and it goes back to it more often than to anything else. The notes list the chosen
  resident's favourites, and hovering over a piece or a find says whose favourite it is.
- **Visitors.** A little while after the house opens, a friend or two from other houses may knock.
  A sweetheart goes to the door, a wallflower finds the furthest seat, a show-off has something to
  show them, and a grump may grumble. Visitors look round at whatever is on show, spend time with
  whoever they are closest to, and after a few minutes go home. They can be asked for things too,
  though never to sleep in anyone's bed.
- **Photos.** The camera stud, or P, saves a picture of the house without anything of the window
  in it, three times the size, wherever you choose.

**Arrange Mode** is chosen from its tab on the cover, so nothing gets moved by accident. Everyone
waits off the furniture while you arrange. Three tabs stand up from the notes:

- **Found things.** Everything the colony has, as small pictures: a red dot on what is shown here,
  a hollow one on what is shown in another household's house. Hover over one to see where it is
  and who found it. Moving something from another house moves it here; nothing is ever shown in
  two places.
- **Furniture.** The starter pieces are always there. More arrive as the colony lives and finds
  things, and stay: a sofa, a long rug, a toy box, a glass case, a bell jar, a curio cabinet and a
  glass counter; and three sets made to go together, the seaside, the woodland and the starlit.
  The chips at the top show one set at a time.
- **Rooms.** Each room's floor and walls. After two weeks a second room can be built, and after
  forty days a third: choose its kind, see the house with it in each place it could go, and build
  it where it looks right. A room other than the first can be taken away again, and everything in
  it goes back to the catalogue and the drawer.

Drag something, or click it, to pick it up, and click to put it down. A ghost shows where it would
go, green where it fits and red where it does not. A doorway slides along its wall the same way.
R or a right-click turns a piece, and Delete puts it back in the catalogue or the drawer; nothing
is ever sold or used up. ⌘Z or Ctrl+Z undoes, and adding Shift redoes. Leaving Arrange Mode hands
the layout back.

Close the window with its stud to end the visit. The window remembers where it was and how big.
Desktop's reduced motion, theme and text size carry over: the notebook is cream by daylight and
charcoal after dark.

### What Home keeps

Home's own data folder holds very little:
- macOS: `~/Library/Application Support/com.Formiga.Formiga-Home`
- Windows: `%APPDATA%\Formiga\Formiga Home\data`
- or wherever `FORMIGA_HOME_DATA_DIR` says.

| File | What it holds |
| --- | --- |
| `window.json` | Where the window was, and how big |
| `open.lock` | Held while a window is open, so a second visit is refused as busy |
| `rehearsals/<colony key>.json` | A rehearsal's layouts. A real visit's layouts are Desktop's to keep |

## Developing

Rust installs itself from `rust-toolchain.toml` (1.97.1, as Desktop pins).

```sh
cargo run -p formiga-home                                   # Desktop's sample household
cargo run -p formiga-home -- --from-save ~/path/to/colony.json --house 1
cargo run -p formiga-home -- --formiga-home <visit directory>
```

- **`--from-save`** reads a Desktop save and never writes to it. It projects the house exactly as
  Desktop would, counting the colony house as 0.
- **`--formiga-home`** is how Desktop starts Home: the visit directory it wrote, as
  [docs/CONTRACT.md](docs/CONTRACT.md) describes.
- **`--home-version`** prints the newest Home version this build reads, and **`--icon <folder>`**
  writes the icon, both for the packaging scripts.

### Packaging

- **macOS.** `scripts/package-macos.sh` builds a universal `Formiga Home.app`, with the bundle id
  and Home version Desktop looks for, and a zip and a disk image beside it in `dist/`.
- **Windows.** `scripts/package-windows.ps1` builds a portable zip and a per-user installer that
  writes the registry values Desktop reads. The installer needs the WiX 4 command-line tool.

Both take the Home version from the binary itself, so a package can never claim another. Both
sign when `FORMIGA_CODESIGN_IDENTITY` (and `FORMIGA_NOTARY_PROFILE`) or
`FORMIGA_SIGNTOOL_CERT_SHA1` is set, and otherwise ship unsigned.

### Renders

The room and every review sheet can be drawn to a PNG without a window. Look at them, cropped and
enlarged, after changing anything visual or any behaviour.

```sh
cargo run -p formiga-home -- --render-room room.png --lived-in --floor floor.checks --wall wall.stripes
cargo run -p formiga-home -- --render-room life.png --lived-in --at 45
```

| Option | Draws |
| --- | --- |
| `--render-room` | The house as it first opens; `--lived-in` furnishes it and shows finds every way they can be shown; `--rooms` grows it to two or three rooms; `--at` runs the household's own life that many seconds |
| `--render-catalog` | Every piece of furniture at every turn |
| `--render-finds` | Everything the colony has, every way it can be shown |
| `--render-poses` | Everyone in every pose Home uses |

`--scale <N>` sets the pixels per scene pixel (3 by default).

The window itself can be pictured too, for checking the notebook: `--snap <PNG>` opens it, waits
`--at` seconds, saves a picture of the window and closes. A window opened for a picture stays
behind whatever else is open and answers to no mouse or key. `--page finds|furniture|rooms` opens
it arranging, `--theme light|dark` sets its theme, and `--rooms` grows a rehearsal's house first.
Give it a scratch `FORMIGA_HOME_DATA_DIR` so the rehearsal it opens is its own.

### Layout

```text
crates/formiga-home-contract/   the household contract: documents, ids, bounds, Desktop's
                                projection of a house, and what Desktop keeps of an answer
crates/formiga-home/src/
  main.rs          arguments, the window, and the renders
  app.rs, app/     the window: Live and Arrange Mode, the notebook round it, the notes page,
                   the menu; and its tests, which drive it headless through egui with pointer
                   and key events
  session.rs       Home's side of a visit: acknowledgement, result, receipt, recall
  host.rs          a visit from Desktop, or a rehearsal standing in for Desktop
  store.rs         the data folder: the window, the lock, rehearsals
  household.rs     the residents, ready to draw, and how they get on
  character.rs     who each resident is, turned into what it feels like doing
  life.rs, life/   the household's life: what each resident does, asked or not, favourites, and
                   visitors coming and going; and its tests
  actor.rs         a resident in the room: walking, poses, cached frames
  house.rs         the rooms set out as one house: walls, doorways, which walls are cut down
  path.rs          finding the way across the house, room to room through the doorways
  arrange.rs       picking up, carrying and putting down; doorways; building rooms; undo
  room.rs          what can go where: footprints, surfaces, walls, the floor
  catalog.rs       every piece of furniture, floor and wall, the sets, the kinds of room
  starter.rs       how a house first looks inside
  scene.rs         one frame of the house, back to front, and what is under the pointer
  iso.rs           the isometric grid, and how big a house's picture is
  art/             the shell, the furniture, shown things, cues, and the notebook
  staging.rs       rooms set up for the review renders and tests
  paint.rs         painting tools
  icon.rs          the app icon: a little room, in pixels
packaging/, scripts/   the macOS app and the Windows installer
```

### Formiga Desktop's crates

`formiga-core`, `formiga-art` and `formiga-travel` all come from Formiga Desktop, from one source
so their types agree. That source is a release tag on GitHub, now `v0.66.6` (travel version 3).
To move to a newer release, change the tag on all three in the root `Cargo.toml` together.

To build against unreleased changes in a local Desktop checkout, create `.cargo/config.toml` (git
ignores it):

```toml
[patch."https://github.com/Von-Van/Formiga-Desktop"]
formiga-core = { path = "../Formiga Desktop/crates/formiga-core" }
formiga-art = { path = "../Formiga Desktop/crates/formiga-art" }
formiga-travel = { path = "../Formiga Desktop/crates/formiga-travel" }
```

Patch all three or none, and delete the file before committing anything that depends on it.

### Checks

The same gate CI runs on macOS and Windows:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`FORMIGA_HOME_BLESS=1 cargo test -p formiga-home-contract --test golden` writes this build's own
version's contract fixtures, and never an older version's.
