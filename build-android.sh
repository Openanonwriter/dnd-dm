#!/usr/bin/env bash
# Builds the Android player APK on Linux (x86_64 or ARM64), without Gradle.
#
#   ./build-android.sh              debug build -> dist/dnd-player.apk
#   ./build-android.sh --release    optimized build (use this one on a phone)
#   ./build-android.sh --install    also install on the attached phone with adb
#
# The app has no Java or Kotlin code, so the APK is only the manifest, two
# resources and libdnd_player.so. This script compiles the library with the
# system clang and lld against the NDK's sysroot, then packs and signs the APK
# with aapt2, zipalign and apksigner from the SDK's build-tools.
#
# Needs:
#   - Rust from rustup with the Android target:  rustup target add aarch64-linux-android
#   - clang, lld, llvm (llvm-ar, llvm-strip), zip, a JDK (java, keytool)
#   - an Android SDK folder ($ANDROID_HOME, default ~/Android/Sdk) holding
#       ndk/<version in app/build.gradle.kts>, platforms/android-<compileSdk>, build-tools/<BUILD_TOOLS>
#   - on a machine that is not x86_64 (Google ships aapt2 and zipalign for
#     x86_64 only): qemu-x86_64-static, and an x86_64 glibc + libgcc unpacked
#     in $X86_ROOT (default ~/Android/x86_64-root). See docs/PLAYER_APP.md.
set -euo pipefail
cd "$(dirname "$0")"
root="$PWD"

release=0; install=0
for a in "$@"; do
    case "$a" in
        --release) release=1 ;;
        --install) install=1 ;;
        -h|--help) sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $a (try --help)" >&2; exit 2 ;;
    esac
done

say() { printf '\033[33m== %s\033[0m\n' "$*"; }
die() { echo "build-android.sh: $*" >&2; exit 1; }

# ---- what to build, read from the Gradle file so there is one place to edit
android="$root/crates/dnd-player/android"
gradle="$android/app/build.gradle.kts"
val() { sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*\"\{0,1\}\([^\"]*\)\"\{0,1\}[[:space:]]*$/\1/p" "$gradle" | head -n1; }
app_id="$(val applicationId)"
min_sdk="$(val minSdk)"
target_sdk="$(val targetSdk)"
compile_sdk="$(val compileSdk)"
version_code="$(val versionCode)"
version_name="$(val versionName)"
ndk_version="$(val ndkVersion)"
for v in app_id min_sdk target_sdk compile_sdk version_code version_name ndk_version; do
    [ -n "${!v}" ] || die "could not read $v from $gradle"
done

# ---- toolchain locations
sdk="${ANDROID_HOME:-$HOME/Android/Sdk}"
build_tools="$sdk/build-tools/${BUILD_TOOLS:-34.0.0}"
ndk="$sdk/ndk/$ndk_version"
ndk_llvm="$ndk/toolchains/llvm/prebuilt/linux-x86_64"
platform_jar="$sdk/platforms/android-$compile_sdk/android.jar"
x86_root="${X86_ROOT:-$HOME/Android/x86_64-root}"
keystore="${ANDROID_KEYSTORE:-$HOME/.android/debug.keystore}"

[ -d "$ndk_llvm/sysroot" ] || die "NDK $ndk_version not found at $ndk"
[ -f "$platform_jar" ] || die "android.jar not found at $platform_jar"
[ -f "$build_tools/lib/apksigner.jar" ] || die "build-tools not found at $build_tools"
for t in clang ld.lld llvm-ar llvm-strip zip java keytool; do
    command -v "$t" >/dev/null || die "$t is not installed"
done
# rustup's cargo, not a distro cargo that has no Android target
if [ -x "$HOME/.cargo/bin/rustup" ]; then export PATH="$HOME/.cargo/bin:$PATH"; fi
command -v rustup >/dev/null || die "rustup is not installed (https://rustup.rs)"
rustup target list --installed | grep -qx aarch64-linux-android \
    || die "Rust Android target missing: rustup target add aarch64-linux-android"

# aapt2 and zipalign are x86_64 programs; run them through qemu elsewhere.
if [ "$(uname -m)" = x86_64 ]; then
    x86() { "$@"; }
else
    command -v qemu-x86_64-static >/dev/null || die "qemu-x86_64-static is not installed"
    [ -e "$x86_root/lib64/ld-linux-x86-64.so.2" ] || die "no x86_64 glibc in $x86_root (see docs/PLAYER_APP.md)"
    x86() { qemu-x86_64-static -L "$x86_root" "$@"; }
fi

# ---- Rust -> libdnd_player.so
say "Rust -> libdnd_player.so (arm64-v8a)"
work="$root/target/android-apk"
rm -rf "$work"; mkdir -p "$work"
runtime="$(ls -d "$ndk_llvm"/lib/clang/*/lib/linux/aarch64 | head -n1)"   # libunwind.a
cat > "$work/cc" <<EOF
#!/bin/sh
exec clang --target=aarch64-linux-android$min_sdk --sysroot="$ndk_llvm/sysroot" -fuse-ld=lld -L"$runtime" "\$@"
EOF
chmod +x "$work/cc"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$work/cc"
export CC_aarch64_linux_android="$work/cc"
export AR_aarch64_linux_android=llvm-ar
if [ "$release" = 1 ]; then profile=release; flag=(--release); else profile=debug; flag=(); fi
cargo build "${flag[@]}" --target aarch64-linux-android -p dnd-player --lib
so="$root/target/aarch64-linux-android/$profile/libdnd_player.so"
[ -f "$so" ] || die "cargo did not produce $so"

# ---- resources + manifest -> APK
say "aapt2 -> APK"
# Gradle takes the package name from "namespace"; aapt2 wants it in the manifest.
sed "s|<manifest |<manifest package=\"$app_id\" |" "$android/app/src/main/AndroidManifest.xml" > "$work/AndroidManifest.xml"
grep -q "package=\"$app_id\"" "$work/AndroidManifest.xml" || die "could not add the package name to the manifest"
x86 "$build_tools/aapt2" compile --dir "$android/app/src/main/res" -o "$work/res.zip"
x86 "$build_tools/aapt2" link -o "$work/unaligned.apk" \
    -I "$platform_jar" --manifest "$work/AndroidManifest.xml" \
    --min-sdk-version "$min_sdk" --target-sdk-version "$target_sdk" \
    --version-code "$version_code" --version-name "$version_name" \
    "$work/res.zip"

mkdir -p "$work/lib/arm64-v8a"
llvm-strip --strip-debug -o "$work/lib/arm64-v8a/libdnd_player.so" "$so"
(cd "$work" && zip -q -X unaligned.apk lib/arm64-v8a/libdnd_player.so)
x86 "$build_tools/zipalign" -f -p 4 "$work/unaligned.apk" "$work/aligned.apk"

# ---- sign (the usual Android debug key, made once; fine for sideloading)
if [ ! -f "$keystore" ]; then
    say "Creating debug key $keystore"
    mkdir -p "$(dirname "$keystore")"
    keytool -genkeypair -keystore "$keystore" -storepass android -keypass android \
        -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10950 \
        -dname "CN=Android Debug,O=Android,C=US" >/dev/null
fi
mkdir -p "$root/dist"
java --enable-native-access=ALL-UNNAMED -jar "$build_tools/lib/apksigner.jar" sign --ks "$keystore" \
    --ks-pass "${ANDROID_KEYSTORE_PASS:-pass:android}" \
    --ks-key-alias "${ANDROID_KEY_ALIAS:-androiddebugkey}" \
    --out "$root/dist/dnd-player.apk" "$work/aligned.apk"
rm -f "$root/dist/dnd-player.apk.idsig"
java --enable-native-access=ALL-UNNAMED -jar "$build_tools/lib/apksigner.jar" verify "$root/dist/dnd-player.apk"

say "Done"
echo "  APK: $root/dist/dnd-player.apk"

if [ "$install" = 1 ]; then
    command -v adb >/dev/null || die "adb is not installed"
    adb install -r "$root/dist/dnd-player.apk"
fi
