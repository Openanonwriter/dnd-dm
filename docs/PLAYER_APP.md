# dnd-player — the character sheet app

One Rust crate (`crates/dnd-player`) that runs on **Android** (NativeActivity,
no Java) and on **Windows/Linux/macOS** (a phone-sized window, handy for
testing). It talks to the DM's `dnd-dm` server over Wi-Fi and shares
`dnd-core` with it, so every number is computed by the same code.

## What it does

- **Find the DM** — "Scan network" probes the local /24 for a dnd-dm server;
  or type the address the DM app shows on its Dashboard.
- **Pick your character** from the campaign's list; the app remembers it.
- **Live sheet** — abilities, saves, skills (tap to roll), AC, initiative,
  speed, HP with damage/heal buttons, death saves, inspiration, attacks with
  roll buttons, spell slots (tap pips), prepared spells, inventory and coins,
  features & traits, notes and roleplay fields.
- **Edits go to the DM** ~0.7 s after you stop typing, through
  `PATCH /api/player/characters/{id}`. Only play-state fields are sent; the
  build (levels, feats, modifiers) and anything the DM hid stay with the DM.
- **Live updates** — a WebSocket to `/ws` tells the app when the DM changes
  anything; it re-fetches the sheet and the encounter. If the socket can't
  connect it polls every 20 s.
- **Party / encounter** — initiative order, whose turn it is, round, a
  read-only battle-map view with tokens; monster HP only if the DM allows.
- **Offline** — the last sheet is cached in the app's data folder and opens
  without the server; edits made offline are kept on the device.
- **Dice** — local roller with history; advantage / disadvantage buttons.
- **Printable sheet** — exports the same HTML the DM app prints.

## Build

Desktop (for testing):

```
cargo run -p dnd-player
```

Set `DND_PLAYER_DATA=<folder>` to choose where config and cache go
(default `%APPDATA%\dnd-player`).

Android:

```
.\build-android.ps1            # debug APK  -> dist\dnd-player.apk
.\build-android.ps1 -Release   # optimized build
.\build-android.ps1 -Install   # also adb install on the attached phone
```

The script expects a Rust Android target (`rustup target add
aarch64-linux-android`), `cargo-ndk` (`cargo install cargo-ndk`), an Android
SDK with NDK r28, JDK 17 and Gradle 8 — by default the portable toolchain
under `C:\dev\xiao-keyboard\tools`; point `ANDROID_HOME`, `JAVA_HOME` and
`GRADLE_BIN` elsewhere if needed. Enable "Install unknown apps" on the phone
and open the APK, or use `-Install` with USB debugging on.

## How it is put together

| file | role |
|------|------|
| `src/lib.rs` | `android_main` entry point; module list |
| `src/main.rs` | desktop entry point |
| `src/app.rs` | `PlayerApp`: screens, event pump, debounce + patch, dice |
| `src/client.rs` | background network thread (`ureq`), WebSocket listener (`tungstenite`), LAN scan |
| `src/store.rs` | `config.toml` and the offline cache |
| `src/ui/*` | connect / pick / sheet screens |
| `android/` | Gradle shell: manifest, icon, no code; `jniLibs/` is filled by cargo-ndk |

Android specifics: the app is a `cdylib` named `libdnd_player.so` loaded by
`android.app.NativeActivity` (`android.app.lib_name = dnd_player`). The
manifest allows cleartext HTTP because the DM server is plain `http://` on
the LAN. Known rough edge: Android's software keyboard works with egui but
some keyboards only deliver Latin characters; the scan button exists so you
rarely need to type an address.
