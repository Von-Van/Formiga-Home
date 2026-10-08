# Changelog

This page records what changed in each version of Formiga Home. Each version's section doubles as
its release notes: `.github/workflows/release.yml` publishes it alongside the downloads when the
version is tagged.

## 0.67.5 (2026-10-08)

Home is built on Formiga Desktop 0.67.5 and reads household version 8, which lets it tell Desktop
who is in the house. It still reads travel version 4 and opens the same houses as before.

- A small card of faces in the corner of the house says who is home, one cell for each resident
  and each friend lent for the visit. Click a face to send them out to the desktop, where they
  live in the colony as usual while the house stays open, and click it again to have them in. A
  resident kept out stays out the next time the house opens. A friend is out on the desktop until
  it comes over and again once it has gone home. With an older Formiga Desktop, everyone is in and
  the card is not shown.

## 0.67.3 (2026-10-06)

Home's version now matches Formiga Desktop's, so the apps released together carry one number. Home
is built on Formiga Desktop 0.67.3 and still reads travel version 4 and household version 7, so it
opens the same houses as before.

- Residents no longer slide across the floor. After about 25 seconds in a house, every walk and
  other looping animation froze on one frame, so residents glided wherever they went. Formiga
  Desktop 0.67.3 fixes this in the drawing Home shares with it.
- Residents walk up onto a seat or a bed and back down off it, a step at a time, instead of
  jumping from the floor beside it and walking out through the furniture.
- A pat or a lift stops a resident's walk, so it answers the pat or dangles while carried rather
  than walking on the spot. Let go over open floor, it lands where it was let go.
- A resident patted while up on a sofa or bed stays up there to answer, and walks down afterwards.

## 0.1.1 (2026-10-06)

- Home is now built on Formiga Desktop 0.67.1, which keeps companions whole on every display. It
  still reads travel version 4 and household version 7, so it opens the same houses as before.
- The Windows installer has been run on Windows.

## 0.1.0 (2026-10-06)

This is the first release of Formiga Home. Formiga Desktop 0.67.0 and later opens a house from its
village in Home, where the household that lives there goes indoors for a while, and comes back out
when the house closes.

- **The house.** Up to three rooms with doorways and a front door, seven floors, seven walls, and
  thirty-two pieces of furniture in three sets, drawn as one cutaway dollhouse.
- **The notebook.** The window is a leather notebook, the house on one page and its notes on the
  other.
- **The residents.** They live on their own, each by their own temperament, with a short list of
  things you can ask of each.
- **Arranging and finds.** Furniture moves and turns, and everything the colony has found can be
  shown, on its own card if it has no picture.
- **Company.** Visitors and favourites, photos, keepsakes and a journal, going next door, and
  friends who stay over.
- **Rehearsal.** Started on its own, Home opens Desktop's sample household or a house from a real
  save, read-only, and checks every layout by the same rules Desktop would.
