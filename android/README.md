# android

The Android keyboard (an `InputMethodService`). It runs the engine (`/engine`, which uses
`../jis-korean/engine` for Korean) as a static library linked into `libuim.so` through a small JNI
wrapper (`app/src/main/cpp/uim_jni.c`) over the engine's C ABI (`engine/include/uim.h`).

## Build

Needs JDK 21, the Android SDK (platform 36, NDK 30.0.16248370, CMake 4.1.2), the Rust Android
targets, and `jis-korean` checked out next to this repo (`~/Projects/jis-korean`):

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
export JAVA_HOME=$(/usr/libexec/java_home -v 21) ANDROID_HOME=$HOME/Library/Android/sdk
./gradlew assembleDebug        # builds the engine for each ABI first (buildEngine)
./gradlew installDebug         # onto the connected device
```

`engine/.cargo/config.toml` points cargo at the NDK's clang for the Android targets
(jis-korean-engine also builds as a cdylib, which needs a linker). If adb's server crashes on
start on macOS, start it with `ADB_MDNS=0 ADB_MDNS_OPENSCREEN=0`.

Then open **uim** on the device: turn the keyboard on, switch to it, pick a bottom row, try it.

## Layout

| File | What |
|---|---|
| `Engine.kt` | The engine's C ABI, through JNI; parses the layout the engine describes |
| `UimInputService.kt` | Applies the engine's output: preedit as composing text, commits, actions; screen class |
| `KeyboardView.kt` | Draws the keys and turns touches into (row, key, direction): flicks, caret drag, ⌫ repeat |
| `SetupActivity.kt` | The launcher screen: enable, switch, bottom-row setting, a test field |

## Not yet (see SPEC.md §8)

KO predictions beyond the "English on the same keys" reading (M4), JA kana-kanji conversion (M6:
the bar offers the kana in both scripts), EN completions in A / a (M7), settings for row height
and the narrow/wide threshold, and calibration (M8). ⇧ in KO/JA wide cycles off → once → locked
on each tap rather than locking on a double tap.
