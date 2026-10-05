# Formiga Home

A separate desktop app that opens one Formiga Desktop household's house as a small isometric
dollhouse, to arrange, show finds in, and spend a little time directing the residents. Read
`docs/DESIGN.md` for the product, where the checkout has it, and
[docs/CONTRACT.md](docs/CONTRACT.md) for the contract with Desktop before changing either side of
it.

- Design documents stay out of git and off GitHub: `docs/DESIGN.md` is kept locally and ignored.
  Keep it current, but never add it, or any other design or planning document, to a commit.

## Relationship to Formiga Desktop

- Desktop lives at `../Formiga Desktop` (GitHub `Von-Van/Formiga-Desktop`, public). Home is
  private. `formiga-core`, `formiga-art` and `formiga-travel` come from Desktop by one release tag
  in the root `Cargo.toml`, so their types agree. Never copy their source into this repository.
- `formiga-home-contract` is a draft of a contract that belongs to Desktop. It lives here until
  Desktop adopts it, then moves to Desktop's workspace, as `formiga-travel` lives there for
  Formiga Hill. Until then, change it as Desktop's own crate would be changed: versioned, with
  its golden fixtures untouched.
- Desktop is authoritative for the colony. Home never reads or writes Desktop's `colony.json` at
  runtime. The only exception is `--from-save`, which reads (never writes) a save and projects it
  as Desktop would.
- Do not edit the Desktop checkout from a Home session. Propose Desktop-side changes instead.

## The household contract

- Home answers a visit once with an acknowledgement, hands back every layout whole as a result
  whenever arranging changes it, and writes one receipt on leaving, all through `session.rs`. A
  recall, or the snapshot disappearing, ends the visit with nothing more written.
- A result changes only the visited household's home, plus taking down elsewhere whatever it now
  shows. A receipt carries only `HomeVisit`, and only when offered. Home never sends prose,
  creature changes or save patches.
- Rehearsals (`--sample`, `--from-save`) stand in for Desktop through the contract's own
  `accept_result`, so a rehearsal proves the same loop a real visit does.

## Design rules that apply to every change

- The same creatures, not lookalikes: draw residents with `formiga-art` from the snapshot's
  `Traveler`, never regenerate one.
- Behaviour is never written for a particular creature. It is read from the snapshot (temperament,
  axes, traits, habits, size, family, bonds) in `character.rs` and `life.rs`, so every household
  lives differently.
- Soft play, not maintenance: drives are reasons, never shown as needs. No neglect, decay,
  currency, streaks or login pressure, and nothing runs while Home is closed.
- Every thing the colony has can be shown, on its card if nowhere else. Never let an item vanish
  because no art exists for it.
- One find, one place: showing it here takes it down elsewhere.
- Rooms follow Formiga's art rules: pixel scale; every material a ramp; light from the upper left;
  edges in a darker shade of their own colour, never black; grain from `paint::noise`. Furniture
  is built from `art::Block`s on its footprint, drawn from the front and behind, and mirrored for
  the other two turns, so every piece must read the same on both sides of its front.
- The whole house is one cutaway (`house.rs`). A wall two rooms share is cut down low and drawn in
  turn with the furniture; so is any far wall that would hide part of another room. Only a
  full-height wall takes a hanging, nothing stands in a doorway, and every house has a front door.
- The notebook round the window (`art/notebook.rs`, `app/notebook.rs`) is painted to the same
  rules and at the same pixel scale as the house; only lettering is egui's. The window has no
  system frame: the cover moves it and its studs close it. Keep the notes short: detail belongs to
  whoever is chosen, or to what is pointed at.
- Whatever stands in front of someone using a piece, or in front of what it shows, goes in the
  sprite's `over` layer. A piece's near corner falls in the middle of its picture, so nothing
  that stands there may hide what is shown on it.
- Reduced motion gets held poses and cuts, not slower animation.

## Conventions

- Rust 1.97.1, edition 2024, matching Desktop. Match Desktop's code style: doc comments that say
  why, plain names, tests named as sentences (`one_find_cannot_be_shown_in_two_houses`).
- The gate, which CI runs on macOS and Windows:
  `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- `--render-room` (with `--lived-in`, `--floor`, `--wall`, `--rooms <n>` and `--at <seconds>`),
  `--render-catalog`, `--render-finds` and `--render-poses` draw without a window. Look at them,
  cropped and enlarged, after changing anything visual or any behaviour in `life.rs`. For the
  notebook itself, `--snap <png>` (with `--page`, `--theme`, `--rooms` and a scratch
  `FORMIGA_HOME_DATA_DIR`) pictures the window and closes; never capture the screen instead.
- Never commit planning material: checklists, roadmaps, "next" lists. Docs describe what exists.
