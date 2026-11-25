# Builds the Android player APK.
#   .\build-android.ps1            -> debug APK
#   .\build-android.ps1 -Release   -> release APK (debug-signed unless you configure signing)
#   .\build-android.ps1 -Install   -> also installs on the attached phone with adb
#
# Needs: Rust target aarch64-linux-android (rustup target add aarch64-linux-android),
# cargo-ndk (cargo install cargo-ndk), an Android SDK with NDK, a JDK 17 and Gradle 8.
# Set the paths below or the environment variables ANDROID_HOME, JAVA_HOME, GRADLE_BIN.
param(
    [switch]$Release,
    [switch]$Install
)
# cargo and gradle print progress on stderr; don't let PowerShell treat that as failure.
$ErrorActionPreference = "Continue"
$root = $PSScriptRoot

# ---- toolchain locations (edit if yours differ)
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { "C:\dev\xiao-keyboard\tools\android-sdk" }
$jdk = if ($env:JAVA_HOME) { $env:JAVA_HOME } else { "C:\dev\xiao-keyboard\tools\jdk" }
$gradle = if ($env:GRADLE_BIN) { $env:GRADLE_BIN } else { "C:\dev\xiao-keyboard\tools\gradle-8.7\bin\gradle.bat" }
$ndkVersion = "28.2.13676358"

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_NDK_HOME = Join-Path $sdk "ndk\$ndkVersion"
$env:ANDROID_NDK_ROOT = $env:ANDROID_NDK_HOME
$env:JAVA_HOME = $jdk
if (-not $env:GRADLE_USER_HOME) { $env:GRADLE_USER_HOME = "C:\dev\xiao-keyboard\tools\gradle-home" }

$android = Join-Path $root "crates\dnd-player\android"
$jni = Join-Path $android "app\src\main\jniLibs"

Write-Host "== Rust -> libdnd_player.so (arm64-v8a)" -ForegroundColor DarkYellow
Push-Location $root
try {
    $profile = @()
    if ($Release) { $profile = @("--release") }
    # note: cargo-ndk 4 takes --platform (its -p is not the platform flag)
    & cargo ndk -t arm64-v8a --platform 26 -o $jni --manifest-path (Join-Path $root "crates\dnd-player\Cargo.toml") build @profile --lib
    if ($LASTEXITCODE -ne 0) { throw "cargo ndk failed" }
} finally { Pop-Location }

Write-Host "== Gradle -> APK" -ForegroundColor DarkYellow
$task = if ($Release) { "assembleRelease" } else { "assembleDebug" }
& $gradle -p $android $task --console=plain -q
if ($LASTEXITCODE -ne 0) { throw "gradle failed" }

$apk = if ($Release) { "$android\app\build\outputs\apk\release\app-release.apk" } else { "$android\app\build\outputs\apk\debug\app-debug.apk" }
$out = Join-Path $root "dist"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item $apk (Join-Path $out "dnd-player.apk") -Force
Write-Host "APK: $out\dnd-player.apk" -ForegroundColor Green

if ($Install) {
    & "$sdk\platform-tools\adb.exe" install -r (Join-Path $out "dnd-player.apk")
}
