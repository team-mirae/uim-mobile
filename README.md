# uim-mobile

**uim** (Universal Input Method) is a touch keyboard for **English, Korean and Japanese** in one
IME. Android first.

The design goal: on a phone the keys are too small to hit reliably, so uim gets bigger targets by
**removing keys, not by predicting**. Predictions are offered on a candidate bar for you to pick;
they never rewrite what you typed. The full design is in [SPEC.md](SPEC.md).

## Layouts

Each language has a **narrow** layout (portrait phones, a foldable's cover screen) and a **wide**
one (from 600 dp: landscape, an unfolded foldable, tablets).

| | Narrow | Wide |
|---|---|---|
| English | QWERTY on bigger keys, and **EN predictive**: 20 keys with a word model resolving the shared ones | QWERTY |
| Korean | 8 × 4 grid with **flick vowels** (consonant flicked = consonant + vowel) | JIS Korean (Mode A) |
| Japanese | 5 × 4 **flick kana** (Gboard's convention) | JIS kana |

Shared on every layout: a top bar (Copy, Paste, candidate bar, Backspace), a symbol layer, and a
bottom row of mode keys. The bottom row is a setting, with idempotent, cycle and E-shift variants
(SPEC §1.2).

**Japanese conversion.** The candidate bar offers kanji conversions of the composition, from a
Viterbi converter over [Mozc](https://github.com/google/mozc)'s open dictionary.

## Repository

| Path | What |
|---|---|
| `engine/` | The Rust engine: modes, layouts, composition, prediction, kana-kanji conversion. C ABI in `include/uim.h`. All behavior lives here. |
| `android/` | The Android shell: draws what the engine describes and sends back (row, key, direction). See [android/README.md](android/README.md). |
| `tools/` | Data builders (`kanji/`, `en-acronyms/`) and the evaluation scripts behind EN predictive's key groups (`phonetic-eval/`). |
| `layouts.html` | Interactive layout drafts. |

Korean composition comes from [jis-korean](https://github.com/team-mirae/jis-korean), which must
be checked out next to this repo (`../jis-korean`).

## Build

```sh
git clone https://github.com/team-mirae/jis-korean
git clone https://github.com/team-mirae/uim-mobile
cd uim-mobile/engine && cargo test       # engine tests, including conversion accuracy
cd ../android && ./gradlew installDebug  # onto a connected device
```

The Android build needs JDK 21, the Android SDK (platform 36, NDK 30.0.16248370, CMake 4.1.2) and
the Rust Android targets; details in [android/README.md](android/README.md).
`engine/.cargo/config.toml` points at the NDK's linkers under one machine's home directory: change
those paths to your NDK.

## Data and licences

The code is MIT-licensed (see [LICENSE](LICENSE)). The data in `engine/data` has its own terms:

| File | Source | Terms |
|---|---|---|
| `kanji.bin` | Mozc's open dictionary (IPAdic, Okinawa dictionary, Google's additions), rebuilt by `tools/kanji/build_kanji.py` | [LICENSE-mozc.txt](engine/data/LICENSE-mozc.txt) |
| `en-acronyms.txt` | Hunspell en_US (SCOWL), built by `tools/en-acronyms/build_acronyms.py` | [LICENSE-scowl.txt](engine/data/LICENSE-scowl.txt) |
| `en-words.txt`, `en-bigrams.txt` | Peter Norvig's word and bigram counts (norvig.com/ngrams) | Not yet settled (SPEC §9, item 10) |

## Status

A working draft. Open questions and the build plan are in
SPEC.md §8–§9.
