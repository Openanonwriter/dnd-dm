#!/usr/bin/env bash
# Builds the DM app and the player app for this Linux machine (x86_64 or
# ARM64) and puts everything needed to run them in one folder.
#
#   ./build-linux.sh              build, then assemble dist/dnd-dm-linux-<arch>/
#   ./build-linux.sh --deps       first install what the build needs (dnf, apt, pacman or zypper; asks for sudo)
#   ./build-linux.sh --desktop    also add both apps to the desktop's application menu
#   ./build-linux.sh --firewall   also open TCP port 8420 so phones can reach the DM app (firewalld; asks for sudo)
#   ./build-linux.sh --test       also run the test suite before packaging
#
# Options can be combined. Needs Rust 1.80 or newer: https://rustup.rs
set -euo pipefail
cd "$(dirname "$0")"

deps=0; desktop=0; firewall=0; tests=0
for a in "$@"; do
    case "$a" in
        --deps) deps=1 ;;
        --desktop) desktop=1 ;;
        --firewall) firewall=1 ;;
        --test) tests=1 ;;
        -h|--help) sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $a (try --help)" >&2; exit 2 ;;
    esac
done

say() { printf '\033[33m== %s\033[0m\n' "$*"; }

if [ "$deps" = 1 ]; then
    say "Installing build and runtime packages"
    if command -v dnf >/dev/null; then
        sudo dnf install -y gcc pkgconf-pkg-config libxkbcommon-devel wayland-devel \
            libX11-devel libXcursor-devel libXi-devel libXrandr-devel \
            mesa-libGL-devel mesa-libEGL-devel mesa-dri-drivers xdg-utils
    elif command -v apt-get >/dev/null; then
        sudo apt-get update
        sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev \
            libx11-dev libxcursor-dev libxi-dev libxrandr-dev libgl1-mesa-dev libegl1-mesa-dev xdg-utils
    elif command -v pacman >/dev/null; then
        sudo pacman -S --needed --noconfirm base-devel pkgconf libxkbcommon wayland libx11 libxcursor libxi libxrandr mesa xdg-utils
    elif command -v zypper >/dev/null; then
        sudo zypper install -y gcc pkg-config libxkbcommon-devel wayland-devel libX11-devel \
            libXcursor-devel libXi-devel libXrandr-devel Mesa-libGL-devel Mesa-libEGL-devel xdg-utils
    else
        echo "No known package manager found. Install a C compiler/linker, libxkbcommon, Wayland or X11 libraries and Mesa by hand." >&2
    fi
fi

if ! command -v cargo >/dev/null; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "Rust is not installed. Get it with:" >&2
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
        echo "then open a new terminal and run this script again." >&2
        exit 1
    fi
fi

if [ "$tests" = 1 ]; then
    say "Running the tests"
    cargo test --workspace
fi

say "Building dnd-dm and dnd-player (release)"
cargo build --release -p dnd-dm -p dnd-player

arch="$(uname -m)"
out="dist/dnd-dm-linux-$arch"
say "Assembling $out"
rm -rf "$out"
mkdir -p "$out"
cp target/release/dnd-dm "$out/dnd-dm"
cp target/release/dnd-player-desktop "$out/dnd-player"
cp -r content "$out/content"
[ -d homebrew ] && cp -r homebrew "$out/homebrew"
[ -f dnd-dm.toml ] && cp dnd-dm.toml "$out/dnd-dm.toml"
mkdir -p "$out/saves" "$out/exports" "$out/docs"
cp docs/PLAYING.md docs/MAPS.md docs/LINUX.md "$out/docs/" 2>/dev/null || true
cp LICENSE "$out/" 2>/dev/null || true
tar -C dist -czf "dist/dnd-dm-linux-$arch.tar.gz" "dnd-dm-linux-$arch"

here="$(cd "$out" && pwd)"

if [ "$desktop" = 1 ]; then
    say "Adding menu entries"
    apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    mkdir -p "$apps"
    cat > "$apps/dnd-dm.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=dnd-dm (Dungeon Master)
Comment=Run a D&D game: characters, maps, encounters, Wi-Fi server for players
Exec=$here/dnd-dm
Path=$here
Terminal=false
Categories=Game;RolePlaying;
EOF
    cat > "$apps/dnd-player.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=dnd-player
Comment=Character sheet and map that connects to a dnd-dm game
Exec=$here/dnd-player
Path=$here
Terminal=false
Categories=Game;RolePlaying;
EOF
    command -v update-desktop-database >/dev/null && update-desktop-database "$apps" >/dev/null 2>&1 || true
fi

if [ "$firewall" = 1 ]; then
    if command -v firewall-cmd >/dev/null; then
        say "Opening TCP port 8420 (firewalld)"
        sudo firewall-cmd --add-port=8420/tcp
        sudo firewall-cmd --permanent --add-port=8420/tcp
    else
        echo "firewalld is not installed; if you use another firewall, allow TCP port 8420 yourself."
    fi
fi

say "Done"
echo "  DM app:      $here/dnd-dm"
echo "  Player app:  $here/dnd-player"
echo "  Archive:     dist/dnd-dm-linux-$arch.tar.gz"
if [ "$firewall" = 0 ] && command -v firewall-cmd >/dev/null; then
    echo "  Phones cannot connect until port 8420 is open: run this script with --firewall,"
    echo "  or: sudo firewall-cmd --add-port=8420/tcp"
fi
