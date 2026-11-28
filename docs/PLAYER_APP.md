# dnd-player — the character sheet app

One Rust crate (`crates/dnd-player`) that runs on **Android** (NativeActivity,
no Java) and on **Windows/Linux/macOS** (a phone-sized window, handy for
testing). It talks to the DM's `dnd-dm` server over Wi-Fi and shares
`dnd-core` with it, so every number is computed by the same code.

## What it does

- **Find the DM** — "Scan network" probes the local /24 for a dnd-dm server;
  or type the address the DM app shows on its Dashboard.
- **Say who you are** — the app asks for your name; the DM sees it on the
  Dashboard and gives you your character. (The DM can instead let players
  pick from the list.)
- **Several games**: the app remembers every game it has joined, one per
  DM's server and campaign, under "Your games" on the connect screen. Each
  keeps its own character, player name, join code and offline copy, so a
  player can be in different campaigns with different DMs (or two campaigns
  the same DM runs from one address). "Join" reconnects, "Open without the
  DM" shows the copy kept on the device, "Forget" removes the game from the
  device only. Files are kept per game in `games/<key>/` of the data folder
  (`store.rs`); the character draft is one per device and can be sent to
  whichever game is joined next.
- **Make your own character** between sessions: species, class, subclass,
  background, ability scores (standard array, point buy or server-rolled),
  skills, class choices (fighting style, invocations…), feats, starting gear,
  spells, and personality traits, ideals, bonds and flaws suggested by your
  background (tap one to use it, or roll). Every pick has an (i) that
  explains it. Options your character doesn't qualify for have a red
  outline and say what they need; the DM can unlock them.
  The builder works before the app has ever reached a DM: the standard
  content (`content/`) is packed into the app when it is built
  (`crates/dnd-player/build.rs`), so "Create a character" is on the connect
  screen from the first start. The draft is kept on the device and uploaded
  after connecting, before the game starts; once connected, the campaign's
  own list replaces the built-in one. Server-rolled ability scores still
  need the DM.
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
- **Help** — a small (i) next to every stat, skill, condition, spell and
  item; tap it for a plain-language explanation.
- **Death** — at 0 HP you roll death saves; after three failures the sheet
  locks (no healing, no rest) until the DM or a revival spell brings you
  back.
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

Android, built on Linux (x86_64 or ARM64, no Gradle):

```
./build-android.sh              # debug build -> dist/dnd-player.apk
./build-android.sh --release    # optimized build (use this one on a phone)
./build-android.sh --install    # also adb install on the attached phone
```

The app has no Java code, so the script compiles `libdnd_player.so` with the
system `clang` and `lld` against the NDK's sysroot, then packs and signs the
APK with `aapt2`, `zipalign` and `apksigner` from the SDK's build-tools.
Package name, SDK levels, version and NDK version are read from
`android/app/build.gradle.kts`. One-time setup:

```
rustup target add aarch64-linux-android
# clang, lld, llvm, zip, a JDK and adb from the distribution, e.g. Fedora:
sudo dnf install clang lld llvm zip cpio java-25-openjdk-headless android-tools

# SDK pieces from https://dl.google.com/android/repository/ (checksums are in
# repository2-3.xml there), unpacked under ~/Android/Sdk (or $ANDROID_HOME):
#   android-ndk-r28c-linux.zip   -> ndk/28.2.13676358/
#   platform-34-ext7_r03.zip     -> platforms/android-34/
#   build-tools_r34-linux.zip    -> build-tools/34.0.0/
```

Google ships `aapt2` and `zipalign` for x86_64 only. On an ARM64 machine the
script runs them through `qemu-x86_64-static` with an x86_64 glibc unpacked
in `~/Android/x86_64-root` (or `$X86_ROOT`):

```
sudo dnf install qemu-user-static-x86
mkdir -p ~/Android/x86_64-root && cd ~/Android/x86_64-root
dnf download --forcearch x86_64 --arch x86_64 glibc libgcc
for f in *.rpm; do rpm2cpio "$f" | cpio -idm; done
mv lib64/* usr/lib64/ && rmdir lib64 && ln -s usr/lib64 lib64 && rm *.rpm
```

The APK is signed with `~/.android/debug.keystore`, created on first use. A
phone refuses to update an app signed with a different key, so an APK built
on another machine has to be uninstalled first (or copy that machine's
`debug.keystore` here, or set `ANDROID_KEYSTORE`).

## How it is put together

| file | role |
|------|------|
| `src/lib.rs` | `android_main` entry point; module list |
| `src/main.rs` | desktop entry point |
| `src/app.rs` | `PlayerApp`: screens, event pump, debounce + patch, dice |
| `src/client.rs` | background network thread (`ureq`), WebSocket listener (`tungstenite`), LAN scan |
| `src/store.rs` | `config.toml` and the offline cache |
| `src/ui/connect.rs` | find and join the DM |
| `src/ui/pick.rs` | the waiting room / character list |
| `src/ui/create.rs` | the character builder |
| `src/ui/sheet.rs` | the live sheet: stats, combat, items, notes, dice |
| `src/ui/spells.rs` | spells tab and the builder's spell section |
| `src/ui/map.rs` | the battle map |
| `android/` | Gradle shell: manifest, icon, no code; `jniLibs/` is filled by cargo-ndk |

Android specifics: the app is a `cdylib` named `libdnd_player.so` loaded by
`android.app.NativeActivity` (`android.app.lib_name = dnd_player`). The
manifest allows cleartext HTTP because the DM server is plain `http://` on
the LAN. Known rough edge: Android's software keyboard works with egui but
some keyboards only deliver Latin characters; the scan button exists so you
rarely need to type an address.
