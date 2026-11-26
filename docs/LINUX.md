# Linux

Both apps build and run on Linux, x86_64 and ARM64 (Fedora, Debian/Ubuntu,
Arch, openSUSE). Wayland and X11 both work.

## Build

```sh
git clone https://github.com/Openanonwriter/dnd-dm.git
cd dnd-dm
./build-linux.sh --deps --desktop --firewall
```

- `--deps` installs the compiler and libraries (asks for sudo). Leave it out
  on later builds.
- `--desktop` adds *dnd-dm* and *dnd-player* to the application menu.
- `--firewall` opens TCP port 8420 in firewalld so phones can connect.
  Fedora blocks it by default.
- `--test` runs the test suite first.

Rust itself comes from <https://rustup.rs> (`curl --proto '=https' --tlsv1.2
-sSf https://sh.rustup.rs | sh`); the script says so if it is missing. The
first build takes a few minutes, longer on a small ARM board.

The result is one folder, `dist/dnd-dm-linux-<arch>/`, plus the same as a
`.tar.gz`:

```text
dnd-dm        the DM app (also the server)
dnd-player    the player app
content/      the rules and content the DM app reads
homebrew/     your own content
saves/        campaigns
```

Keep the folder together: `dnd-dm` looks for `content/` next to itself. To
keep the data somewhere else, set `DND_DM_HOME=/path/to/folder`.

## Run

```sh
dist/dnd-dm-linux-$(uname -m)/dnd-dm
dist/dnd-dm-linux-$(uname -m)/dnd-player
```

The player app keeps its settings in `~/.local/share/dnd-player`
(`$XDG_DATA_HOME` is respected).

## Fedora on ARM

Packages installed by `--deps`:

```sh
sudo dnf install gcc pkgconf-pkg-config libxkbcommon-devel wayland-devel \
    libX11-devel libXcursor-devel libXi-devel libXrandr-devel \
    mesa-libGL-devel mesa-libEGL-devel mesa-dri-drivers xdg-utils
```

The apps draw with OpenGL (or OpenGL ES). If a window opens black or the app
exits at start with a graphics error, the board's GPU driver is the likely
cause; this runs it on the CPU instead, which is fast enough:

```sh
LIBGL_ALWAYS_SOFTWARE=1 ./dnd-dm
```

If the window fails to open under Wayland, try X11: `WAYLAND_DISPLAY= ./dnd-dm`.

## Phones cannot find the DM

1. Same Wi-Fi network for both.
2. Port open: `sudo firewall-cmd --list-ports` should show `8420/tcp`. Open
   it with `sudo firewall-cmd --add-port=8420/tcp` (add `--permanent` and
   run it again to keep it after a reboot).
3. In the DM app the top bar shows the address to type on the phone if the
   scan finds nothing.

## Windows and Linux together

Campaign folders (`saves/<name>/`) and content are plain text and identical
on both systems: copy them across as they are. A DM app on Linux serves
Windows and Android players and the other way round.
