# Formiga Home

Formiga Home is a small, optional companion to Formiga Desktop. Desktop shows a colony of
creatures living their lives around the edges of your screen, and that colony has a village of
little houses. Home is what you see when you open one of those houses: the inside of it, drawn as
a small isometric dollhouse on the page of a notebook, with the household that lives there going
about its day.

The idea is fairly simple. Desktop is their life around you, and Formiga Hill is the place you go
together, but neither one lets you step inside. Home does. You can arrange the rooms, set out
things the colony has found, let the house grow a room or two over time, and spend a few minutes
directing the residents more closely than Desktop ever does. When you stop directing them, they
simply get on with their own lives.

## What it offers

- **The same creatures, not lookalikes.** Every resident is drawn from exactly what Desktop sends
  about it, and behaves according to its own temperament, habits, size, family and friendships.
  Because nothing is scripted for a particular creature, no two households live quite the same
  way.
- **Found things become physical.** Anything in the colony's scrapbook, and any souvenir brought
  back from Formiga Hill, can be set on a shelf, put in a case, stood on a table or the floor, or
  hung on a wall. Even something without its own picture still gets a card, so nothing the colony
  owns is ever left out.
- **Friends drop by.** Desktop can lend a household one or two of its closest friends from other
  houses for the visit. They knock at the front door, look round at what is on show, and go home
  again, sometimes leaving a small gift behind. A close friend may ask to stay over, or be asked,
  and then stays as long as the house is open and sleeps in a spare bed.
- **Next door.** From one house you can go over to another in the village. Desktop opens it next,
  if it will; nothing else about the colony changes.
- **Keepsakes of its own.** Beyond the colony's finds, a house collects a few things that only
  happen at home: a gift from a visitor, a drawing by one of the little ones, or a framed copy of
  a photo you took.
- **A journal.** The notebook keeps short notes on what has happened in the house: who visited,
  what someone took a liking to, when a room was built.
- **A house that grows.** Over time a house can gain a second and third room, all seen together
  in one cutaway, with doorways between them.
- **Day and night.** The house follows your own clock: as drawn by day, warm as the light goes,
  and blue at night but for the pools of its lamps, when everyone tires sooner and sleeps longer.
  The garden through the front door, and what comes in on the doormat, follow the year.
- **Soft play.** There are no needs to keep up, no money, no decay, and nothing is lost by staying
  away. Nothing runs while Home is closed.

### What Home does not do

Desktop stays the home of the colony, and Home is careful to stay a guest. It never reads or
writes the colony's save file, and it never changes a creature. All it hands back to Desktop is
the arrangement of the houses (with their keepsakes and journals), a note that you visited, and,
only if Desktop asks for them, a few small hints about who spent time together and what moments
might be worth remembering. Desktop then checks everything it is given and keeps only what holds
up.

This boundary is probably the most important design decision in the project. It means Home can
be missing, crash, or be out of date without putting the colony at risk: if anything goes wrong,
the household simply walks back out onto the desktop, and the layout Desktop last kept still
stands. The full agreement between the two apps is written up in
[docs/CONTRACT.md](docs/CONTRACT.md).

## Where things stand

Home is released. Formiga Desktop opens houses in it from 0.67.0: install Home, and clicking a
house in Desktop's village opens it here. Started on its own, Home runs in **rehearsal**: it opens
either Desktop's own sample household or a house from a real colony save, read-only. A rehearsal
is not a shortcut, though. It keeps its layouts in Home's own folder and checks them with the very
same rules Desktop would use, so a rehearsal proves the same loop a real visit would.

| Part | Status |
| --- | --- |
| The house: up to three rooms, doorways and a front door, seven floors, seven walls, thirty-two pieces of furniture in three sets plus the house's own | Released |
| The notebook window: the leather cover as its frame, the house on one page and notes on the other | Released |
| Residents living on their own, and the short list of things you can ask of each | Released |
| Arranging, Found Things, and the card for anything without its own picture | Released |
| Visitors, favourites and photos | Released |
| Keepsakes, the Journal, and souvenirs each kept their own way | Released |
| Going next door, and friends staying over | Released |
| The household contract (`formiga-home-contract`) | Desktop's, since Desktop 0.67.0; Home takes it by Desktop's release tag |
| Opening a house from Desktop's village | In Desktop from 0.67.0 |
| Packaging for macOS | Run end to end: a universal, signed `Formiga Home.app` with Desktop's bundle id and contract version, a zip and a disk image, each with its checksum; the packaged app answers a visit from a session directory |
| Packaging for Windows | A per-user installer and a portable zip, each with its checksum; the installer has been run on Windows |

## A visit

The window is a notebook. Its leather cover acts as the frame: drag it to move the window, and use
the brass studs to close it, put it away, open it wide or take a photo. The house is drawn on the
left-hand page and notes are kept on the right. "How to…" at the foot of the notes lists
everything the mouse and keys can do, so the walk-through below is a tour rather than a manual.

### Living in the house

The notebook opens in **Live Mode**, where the household is simply at home.

- **Choose someone** by clicking a resident or its portrait in the notes. The notes then say a
  little about them: what they are like, what they are doing, what they have come to like, and
  what they have been asked to do.
- **Ask for something** by clicking anything in the room while someone is chosen. What is on
  offer depends on what was clicked. A seat can be sat in or napped on, a bed can be slept or
  bounced on, a toy can be played with alone or with someone, and a find on a shelf can be looked
  at, shown off, or (if it is small enough) tried on for a while. Clicking another resident or a
  visitor offers things like saying hello, playing, teasing or a hug, depending on their
  temperaments and how well they get on. Clicking the floor sends them there, through doorways if
  need be.
- **Each resident holds up to three requests at a time.** The notes show the list, and a ✕ takes
  one back. Dragging a resident to pick it up, or right-clicking it for a pat, clears the list.
- **Leave them be**, and they decide for themselves. Over time, whatever a resident keeps
  choosing (a seat, a bed, a toy, a find) becomes its favourite, and it returns there more often.
  Hovering over a piece of furniture or a find says whose favourite it is.
- **Visitors** may knock a little while after the house opens. How the household greets them
  comes from who they are: a sweetheart goes to the door, a wallflower finds the furthest seat, a
  show-off has something to show, and a grump may grumble. Visitors can be asked for things too,
  though never to sleep in someone else's bed.
- **Staying over.** Click a visitor, or a visitor with someone chosen, and ask them to stay over.
  A close friend agrees; a shy one only if it is very close. A friend whose visit is up may ask to
  stay of its own accord. Someone staying over stays until you leave, and when tired sleeps in the
  guest bedroll, or in a bed nobody at home has made their favourite.
- **Moving in.** A close friend can be asked, from its menu, to come and live here, once a visit.
  If it would like to, Desktop hears of it when you leave and decides, by its own rules; Home
  never moves anyone itself.
- **Next door.** The household page lists the village's other houses. Click one to go over:
  Desktop closes this house and opens that one in the same place on screen. A rehearsal opens it
  itself, in the same window.
- **The Journal** tab in the notes shows what has happened in the house, newest first, grouped
  by day.
- **Getting closer.** − / Fit / + in the corner of the house's page, the + and − keys, or a pinch
  bring a large house closer, a whole pixel at a time so the art stays crisp. Scroll, or drag
  across the floor, to move around.
- **Photos.** The camera stud, or P, saves a picture of the house at three times its size,
  wherever you choose, without any of the window in it. A framed copy also goes into the house's
  drawer as a keepsake.

### Arranging

**Arrange Mode** is chosen from its tab on the cover, so nothing gets moved by accident. While you
arrange, everyone waits off the furniture. Three tabs stand up from the notes:

- **Found things** shows everything the colony has, along with the house's own keepsakes. A red
  dot marks what is shown here, and a hollow one marks what is shown in another household's
  house. Moving something from another house moves it here, since one find can only be in one
  place at a time.
- **Furniture** always has the starter pieces. More arrive as the colony lives and finds things,
  and they stay: a sofa, a long rug, a toy box, a glass case, a bell jar, a curio cabinet, a glass
  counter, and three matching sets (seaside, woodland and starlit). A guest bedroll is there from
  the start, for whoever stays over.
- **Rooms** sets each room's floor and walls. After two weeks a second room can be built, and
  after forty days a third. You choose its kind, preview the house with it in each place it could
  go, and build it where it looks right. Any room but the first can be taken away again, and its
  contents go back to the catalogue and the drawer.

Drag or click something to pick it up, and click to put it down. A ghost shows where it would go,
green where it fits and red where it does not. Doorways slide along their walls the same way. R
or a right-click turns a piece, and Delete returns it to the catalogue or drawer. Nothing is ever
sold or used up. ⌘Z or Ctrl+Z undoes, and adding Shift redoes. Leaving Arrange Mode hands the new
layout back to Desktop.

The notebook answers to its keys as well as the mouse: L and A for living in the house and
arranging it, [ and ] to turn the notes' pages, N (and Shift+N) to choose the next one in the
house, P for a picture, + − and 0 to come closer and fit again, H for the help note, and Escape to
put down or close whatever is open.

Closing the window with its stud ends the visit. The window remembers its size and position, and
Desktop's reduced motion, theme and text size carry over, so the notebook is cream by daylight and
charcoal after dark. Larger text makes the window open, and stay, larger, so the notebook keeps
its room.

### What Home keeps on your computer

Very little. Home's data folder is:

- macOS: `~/Library/Application Support/com.Formiga.Formiga-Home`
- Windows: `%APPDATA%\Formiga\Formiga Home\data`
- or wherever `FORMIGA_HOME_DATA_DIR` points.

| File | What it holds |
| --- | --- |
| `window.json` | Where the window was, and how big |
| `open.lock` | Held while a window is open, so a second visit is turned away as busy |
| `rehearsals/<colony key>.json` | A rehearsal's layouts. A real visit's layouts belong to Desktop |

## Developing

This section is for anyone building or changing Home. Rust installs itself from
`rust-toolchain.toml` (1.97.1, the same version Desktop pins).

```sh
cargo run -p formiga-home                                   # Desktop's sample household
cargo run -p formiga-home -- --from-save ~/path/to/colony.json --house 1
cargo run -p formiga-home -- --formiga-home <visit directory>
```

- **`--from-save`** reads a Desktop save and never writes to it. It builds the house exactly as
  Desktop would, counting the colony house as 0.
- **`--formiga-home`** is how Desktop itself starts Home, pointed at the visit directory Desktop
  wrote, as [docs/CONTRACT.md](docs/CONTRACT.md) describes.
- **`--home-version`** prints the newest contract version this build reads, and
  **`--icon <folder>`** writes the app icon. Both exist for the packaging scripts.

### Seeing changes without a window

Because so much of Home is visual, most changes are best checked by looking at them. Home can draw
the room and several review sheets straight to PNG files, without opening a window. It helps to
view them cropped and enlarged after changing anything visual, or anything about how residents
behave.

```sh
cargo run -p formiga-home -- --render-room room.png --lived-in --floor floor.checks --wall wall.stripes
cargo run -p formiga-home -- --render-room life.png --lived-in --at 45
```

| Option | Draws |
| --- | --- |
| `--render-room` | The house as it first opens. `--lived-in` furnishes it and shows finds and keepsakes every way they can be shown, `--rooms` grows it to two or three rooms, and `--at` lets the household live that many seconds first |
| `--render-catalog` | Every piece of furniture, at every turn |
| `--render-finds` | Everything the colony has, then a lived-in house's keepsakes, every way each can be shown |
| `--render-poses` | Every resident in every pose Home uses |

`--scale <N>` sets how many screen pixels make one pixel of the art (3 by default).
`--text <PERCENT>` sets a rehearsal's text size, 100 to 150. `--hour <H>` and `--month <M>` show
the house at that hour and in that month; a picture is
otherwise drawn at midday in June, so it is the same every time, and the window follows the clock.

The notebook itself can be pictured too. `--snap <PNG>` opens the window, waits `--at` seconds,
saves a picture of just that window and closes. A window opened this way stays behind everything
else and ignores the mouse and keyboard. `--page finds|furniture|rooms` opens it in Arrange Mode
and `--page journal` on the Journal, `--theme light|dark` sets its theme, `--zoom <steps>` brings
the house that many pixels closer, `--lived-in` starts a rehearsal that has no house yet in the
lived-in one, and `--rooms` grows a rehearsal's house first. Give it a scratch `FORMIGA_HOME_DATA_DIR` so the
rehearsal it opens is its own.

### Packaging

- **macOS.** `scripts/package-macos.sh` builds a universal `Formiga Home.app`, with the bundle id
  and contract version Desktop looks for, plus a zip and a disk image beside it in `dist/`.
- **Windows.** `scripts/package-windows.ps1` builds a portable zip and a per-user installer that
  writes the registry values Desktop reads. The installer needs the WiX 4 command-line tool.

Both scripts read the contract version from the built binary itself, so a package can never claim
a version it does not support. Pushing a `v*` tag runs both on GitHub's own macOS and Windows
machines and publishes a release with every package and its checksum. Both sign when `FORMIGA_CODESIGN_IDENTITY` (and
`FORMIGA_NOTARY_PROFILE`) or `FORMIGA_SIGNTOOL_CERT_SHA1` is set, and otherwise ship unsigned.

### How the code is laid out

```text
crates/formiga-home/src/
  main.rs          arguments, the window, and the renders
  app.rs, app/     the window: Live and Arrange Mode and the pointer in each, the keys, the
                   zoom, the notebook round it, the notes pages, the menu, photos, visits and
                   going next door; and its tests, which drive it headless with pointer and key
                   events
  session.rs       Home's side of a visit: acknowledgement, result, receipt, recall
  host.rs          a visit from Desktop, or a rehearsal standing in for Desktop
  store.rs         the data folder: the window, the lock, rehearsals
  household.rs     the residents, ready to draw, and how they get on
  character.rs     who each resident is, turned into what it feels like doing
  life.rs, life/   the household's life: what each resident does, asked or not, favourites, and
                   visitors coming and going; and its tests
  keepsakes.rs     gifts, drawings and framed photos the household comes by at home
  journal.rs       how the Journal page words what has happened
  actor.rs         a resident in the room: walking, poses, cached frames
  house.rs         the rooms set out as one house: walls, doorways, which walls are cut down
  path.rs          finding the way across the house, room to room through the doorways
  arrange.rs       picking up, carrying and putting down; doorways; building rooms; undo
  room.rs          what can go where: footprints, surfaces, walls, the floor
  catalog.rs       every piece of furniture, floor and wall, the sets, the kinds of room
  starter.rs       how a house first looks inside
  scene.rs         one frame of the house, back to front, and what is under the pointer
  iso.rs           the isometric grid, and how big a house's picture is
  art/             the shell, the furniture, shown things, souvenirs, keepsakes, cues, and the
                   notebook
  staging.rs       rooms set up for the review renders and tests
  paint.rs         painting tools
  icon.rs          the app icon: a little room, in pixels
packaging/, scripts/   the macOS app and the Windows installer
```

### Formiga Desktop's crates

`formiga-core`, `formiga-art`, `formiga-travel` and `formiga-home-contract` all come from Formiga
Desktop. They are taken from one release tag on GitHub, currently `v0.67.3` (travel version 4,
household version 7), so that their types always agree with each other. Moving to a newer release
means changing the tag on all four together in the root `Cargo.toml`, which
`scripts/set-version.sh <version> <tag>` does along with Home's own version and `Cargo.lock`. Each
version's release notes are its section of [CHANGELOG.md](CHANGELOG.md).

To build against unreleased changes in a local Desktop checkout, create `.cargo/config.toml` (git
ignores it):

```toml
[patch."https://github.com/Von-Van/Formiga-Desktop"]
formiga-core = { path = "../Formiga Desktop/crates/formiga-core" }
formiga-art = { path = "../Formiga Desktop/crates/formiga-art" }
formiga-travel = { path = "../Formiga Desktop/crates/formiga-travel" }
formiga-home-contract = { path = "../Formiga Desktop/crates/formiga-home-contract" }
```

Patch all four or none, and delete the file before committing anything that depends on it.

### Checks

CI runs the same three checks on macOS and Windows:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The household contract, with its tests and its saved example documents for every version (its
"golden fixtures"), lives in Formiga Desktop's workspace, and changes there.
