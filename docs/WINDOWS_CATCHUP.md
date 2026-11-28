# Windows: catching up with the Linux and Android builds

Written 2026-11-27 on the Linux machine. Everything after commit `9809ce6`
("README: one-word captions…") was made and tested on Linux (ARM64) and on
an Android phone. **Nothing since then has been built or run on Windows.**
The code does type-check for Windows (`cargo check --target
x86_64-pc-windows-gnu -p dnd-dm -p dnd-player` passes on the Linux machine),
but that is not a build and not a run. This is the list of what to do and
what to look at on the Windows machine. Delete this file when it is done.

## 1. Get the code

```powershell
git fetch origin
git status
```

If `git status` says the branch has diverged (the history of `master` was
rewritten once on 2026-11-26), and there is no local work to keep:

```powershell
git branch backup-before-catchup
git reset --hard origin/master
```

If there *is* local work, stop and sort that out first; `reset --hard`
throws uncommitted changes away.

## 2. Build and test

```powershell
cargo test --workspace
cargo build --release --workspace
cargo build --release -p dnd-player --example demo
.\build-android.ps1 -Release
```

Expect 62 tests. Close the apps first: a build fails with "Access is
denied" while the `.exe` it replaces is running.

Things in the build that are new and have only run on Linux:

- `crates/dnd-player/build.rs` reads `content\` at build time and packs the
  character-builder content into the player app (offline character
  creation). It finds the folder as `CARGO_MANIFEST_DIR\..\..\content`.
- Both apps compile in a font, `crates\dnd-mapview\assets\fonts\AdwaitaSans-Regular.ttf`
  (`include_bytes!`). `.gitattributes` must not turn it into text: check the
  file is 879,796 bytes after checkout.
- New examples: `cargo run --release -p dnd-player --example table`
  (a practice table on port 8421 where the monsters play themselves).

## 3. The Android signing key

The phone now has an APK built on Linux, signed with the Linux machine's
debug key. An APK from `build-android.ps1` is signed with the Windows
machine's debug key, and the phone will refuse it
(`INSTALL_FAILED_UPDATE_INCOMPATIBLE`). Pick one:

- copy `~/.android/debug.keystore` from the Linux machine over
  `%USERPROFILE%\.android\debug.keystore` on Windows (then both machines
  can update the phone), or
- uninstall the app on the phone before installing the Windows build (the
  app's saved games list and draft character go with it).

`build-android.ps1` itself was not changed. `build-android.sh` is the Linux
way to build the same APK (no Gradle); it has no Windows counterpart and
does not need one.

## 4. What changed, to look at on Windows

Both apps were restyled onto one design system (see "The look" in
`docs/DEVELOPING.md` and `docs/PLAYER_APP.md`). On Windows, check these in
particular, because they depend on display scaling and fonts:

- **Display scaling (125 %, 150 %).** Controls in the DM app are now 30
  points tall (were 24) and rows are spaced wider, so every page is longer.
  Look at each tab at 125 % and 150 % for anything cut off or pushed past
  the window edge:
  `DND_DM_SHOT=shot.bmp`, `DND_DM_VIEW=…`, `DND_DM_SHOT_ZOOM=1.25` / `1.5`
  (see `docs/DEVELOPING.md`). On Linux this was checked at 1440x900,
  1366x768 and 900x600 at zoom 1 only.
- **The font.** Text is Adwaita Sans with egui's own fonts behind it for
  symbols. Look for boxes where an icon should be, on buttons and in the
  tab bars of both apps.
- **Nothing may sit on top of text.** That is a rule now: a button never
  covers words (the (i) pop-up is the one exception). Long item names,
  spell lines such as Alarm, and narrow windows are where it would show.
- **DM app:** the player rules (pick own character, move own token, attacks
  change hit points, enforce reach and range, see monster HP, may break the
  rules, start level) moved from the Dashboard to **Campaign → Rules for
  players**. Conditions are in aligned columns. The spell list's Effect
  column is cut with "…" at the panel edge.
- **Player app (desktop window and phone):** hit points and armor class in
  the header; at most six places in the bottom bar (Dice, Features and
  Settings are under More); scores, saves and skills roll when tapped; roll
  results float above the bottom bar; several saved games on the connect
  screen (one per server and campaign); the character builder works before
  ever connecting.
- **Player data folder.** Per-game files moved to
  `%APPDATA%\dnd-player\games\<key>\`; files from older versions directly in
  `%APPDATA%\dnd-player\` are still read. Open the desktop player once and
  check the old character still shows.

## 5. Docs that still speak Windows-only

`docs/DEVELOPING.md` and `docs/PLAYER_APP.md` name `.exe` files and
PowerShell for the screenshot tools; those parts are unchanged and should
still be right. `docs/LINUX.md` and `build-linux.sh` are the Linux side.
One Linux trap worth knowing: `build-linux.sh` deletes and recreates
`dist/dnd-dm-linux-<arch>/`, including a campaign saved inside it.

## 6. When it all works

- Rebuild the Windows release the usual way.
- Retake any README screenshot that looks different on Windows (the ones in
  `docs/img` were regenerated on Linux on 2026-11-27).
- Delete this file and commit.
