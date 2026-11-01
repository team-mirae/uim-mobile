# uim-mobile — Universal Input Method (mobile)

A touch keyboard for English, Korean and Japanese in one IME. Android first.

Status: **draft v0.15** (2026-11-01: v0.2 adds EN narrow and the 10 × 4 symbol layer; v0.3 adds the reserved E key and makes K and J 1.5u; v0.4 drops the 13th number-row key from EN wide and KO wide so their key area is a straight 12 × 4 grid; v0.5 designs KO narrow as an 8 × 4 grid of equal keys with flick vowels, and adds flick as KO narrow's exception to Principle 1; v0.6 adds Hi (F6), Ka (F7) and a っ key to JA wide, making it a 13 × 4 grid; v0.7 designs JA narrow as a 5 × 4 flick layout after Gboard; v0.8 replaces J with ひ and カ script modes, makes K 1u, and turns JA wide's Hi / Ka into the same mode keys; v0.9 adds four alternative bottom rows as a setting, relabels the Korean mode key K as 한, and drops Shift+Z = F10 in KO wide; v0.10 drafts EN predictive as a 24-key sound layout; v0.11 adds its consonant-flick variant as a setting; v0.12 adds QWERTY groups as a second EN predictive candidate; v0.13 adds a near-exact third candidate with a capitalize-previous-word key; v0.14 adds a 20-key fourth candidate with only the rare letters grouped, its groups chosen by search as ZQP XJK VBY; v0.15 makes that layout EN predictive (§3.3) and keeps the others as alternatives considered (§3.4)). Items marked **[decide]** are not settled. Where this spec gives
a default, implement the default and say so; don't guess silently.

---

## 0. Problem and principles

**Problem.** On a phone the keys are too small to hit reliably.

What has been tried:

| Tried | Result |
|---|---|
| `team-mirae/flick-korean` (12-key flick) | Too tiring: every character is a swipe direction. |
| flick-korean 초성입력모드 (initial-consonant prediction) | Too inaccurate: the word you want often never shows up. |

**Principles**

1. **Tap only, except the narrow KO and JA layouts.** No character needs a swipe, flick or
   long-press, with one exception: **KO narrow (§4.2) and JA narrow (§4.4) use 4-way flicks**
   (N/E/S/W) on their keys, which is how they fit everything into a phone-width grid. Elsewhere
   gestures may be added later as shortcuts, but every character can always be typed with taps
   alone. Flicks are deterministic too: the same key and direction always give the same output.
2. **Bigger targets come from removing keys, not from predicting.** The narrow Korean layout makes its keys bigger
   by folding 2-set's (두벌식) vowel half into 13 flick keys, so 32 keys do the work of 2-set
   plus every compound vowel. It does not depend on the predictor guessing
   right. Composition is deterministic: the same taps always give the same text.
3. **Predictions are something you can pick, never a correction.** The candidate bar suggests; it never rewrites
   what you typed.
4. **All behavior lives in the engine** (as in `jis-korean` and `flick-korean`). The Android shell
   turns touches into key ids, draws the labels it gets from the engine, and applies the engine's output. A
   shell that decides what a key types is a bug.
5. **Offline.** No `INTERNET` permission. Dictionaries and learned data stay on the device.

**Inspiration (prior art the layouts draw on)**

- **JIS Korean** (`team-mirae/jis-korean`, the owner's own layout). Its Mode A is the wide Korean
  layout as it stands, and its engine is reused unchanged.
- **The JIS kana layout** (JIS X 6002). Every kana on its own key, so no flicks and no multi-tap.
  This is the wide Japanese layout.
- **Japanese PC IMEs (ATOK, MS-IME, Mozc).** A conversion candidate bar on a full-key layout.
- **Big-key "quick" phone keyboards** (단모음 keyboards, Gboard 五段 / Godan). They make keys bigger by
  splitting the alphabet into separate consonant and vowel groups instead of overloading keys.
  The narrow Korean layout follows this idea.

---

## 1. Grid and units

- The keyboard is a grid **13u wide**. `u = keyboard width / 13`.
- Every layout has **6 rows of equal height**: the top bar, four key rows, and the bottom row.
  The row height is a setting (default 7.5 mm, in physical units through `DisplayMetrics.xdpi/ydpi`).
- **No dead zones.** A touch goes to the key whose rectangle contains it. A touch in a gap goes to the
  nearest key centre. No area ignores touches.
- Rows with fewer than 13u of keys stretch their keys evenly to fill the 13u. **[decide]** Option:
  a JIS-style stagger instead. The default is stretch, because stagger leaves edge area unused.

### 1.1 Shared rows

The top bar and the bottom row are **the same in every layout and mode** (the bottom row can be swapped in settings, §1.2):

```
Top:    [Copy 1u][Paste 1u][ Candidates 8u ][ Backspace 3u ]
Bottom: [S 1u][E 1u][A 1u][a 1u][ Space 3u ][ 한 1u ][ ひ 1u ][ カ 1u ][ Enter 3u ]
```

| Key | Behavior |
|---|---|
| Copy | `InputConnection.performContextMenuAction(android.R.id.copy)`. If nothing is selected, it does nothing (light haptic). **[decide]** Copy the whole field instead? |
| Paste | `performContextMenuAction(android.R.id.paste)`. Flushes the composition first. |
| Candidates | Up to 4 candidates in 8u (§6). Tap one to commit it. |
| Backspace | Deletes one unit through the engine: a jamo from the syllable being composed, otherwise a whole character (in KO, a finished syllable goes as one block, as in the JIS Korean automaton). Hold to repeat (400 ms delay, then 50 ms). |
| S | Opens the symbol layer (§5): page 1, then page 2, then closed. |
| E | Switches to **EN predictive** (§3.3): English spelled on 20 big keys, with a language model resolving the shared keys. **Draft.** |
| A | Switches to English upper case (latched). |
| a | Switches to English lower case. |
| Space | Space. In KO and JA it commits the composition first, as the engine says. |
| 한 | Switches to Korean. The screen class (§2) picks narrow or wide. |
| ひ | Switches to Japanese, **hiragana** script. The screen class picks narrow or wide. |
| カ | Switches to Japanese, **katakana** script. |
| Enter | Commits the composition, then runs the editor's `imeOptions` action (send, search, go, next, done) or inserts a newline. The label follows the action. |

Switching modes **flushes** the current composition (`jk_flush` or the equivalent).
The active mode key is highlighted.

**ひ and カ work like A and a.** They're one Japanese mode with two scripts, the way A and a are
one English mode with two cases:

- Kana keys type the active script: ひ types hiragana, カ types full-width katakana. The key
  legends follow (か in ひ, カ in カ), as letter legends follow A and a.
- Switching between ひ and カ **doesn't flush**: kana already being composed keep their script, and
  the next kana uses the new one. Switching to or from any other mode flushes as usual.
- **Prediction uses both scripts in both modes.** The candidate bar (§6) offers hiragana,
  katakana and kanji candidates either way, but the active script is **weighted more heavily**,
  so katakana words rank higher in カ and hiragana and kanji rank higher in ひ.

### 1.2 Alternative bottom rows (setting)

The bottom row in §1.1 is the **default**. Settings offers four alternatives. The choice applies to
every layout and mode, in both screen classes; only the bottom row changes.

| Setting | Bottom row | Widths |
|---|---|---|
| **Idempotent, KO priority** (default) | `[S][E][A][a][ Space ][한][ひ][カ][ Enter ]` | 1 1 1 1 3 1 1 1 3 |
| **Idempotent, JA priority** | `[S][E][A][a][ Space ][ひ][カ][한][ Enter ]` | 1 1 1 1 3 1 1 1 3 |
| **Cycle, A** | `[한A1][あA1][A][      Space      ][ Enter ]` | 1.5 1.5 1 6 3 |
| **Cycle, a** | `[한A1][あA1][a][      Space      ][ Enter ]` | 1.5 1.5 1 6 3 |
| **Cycle, E** | `[한A1][あA1][E][      Space      ][ Enter ]` | 1.5 1.5 1 6 3 |
| **E-shift, KO priority** | `[S][ E/⇧ ][ , ][ Space ][한][ひ][カ][ Enter ]` | 1 1.5 1.5 3 1 1 1 3 |
| **E-shift, JA priority** | `[S][ E/⇧ ][ , ][ Space ][ひ][カ][한][ Enter ]` | 1 1.5 1.5 3 1 1 1 3 |

**Idempotent rows** give every mode its own key, so a key always lands on the same state and
pressing it again changes nothing. The two differ only in order: **KO priority** puts 한 first
(한 ひ カ), and **JA priority** puts the Japanese keys first (ひ カ 한). Labels and dimensions are
the same in both.

**E-shift rows** drop A and a: English is **EN predictive only** (§3.3), with no EN a / A and
no caps lock.

- **E/⇧** shows **E** outside EN predictive and switches to it. In EN predictive it shows **⇧**:
  a tap makes the **next letter** upper case (a second tap cancels it), and **holding** it types
  upper case for as long as it's held (a hold with another key pressed sends no tap).
- **,** taps `,`; flicks **E `.`**, **W `@`**, **N `-`** (as §4.2 flicks).
- The rest is the matching idempotent row.
- **Case in prediction** is only "all caps or not". When every letter of the word so far (2+) was
  typed upper case, acronyms rank first and candidates show in all caps (A·ZQP·I is API). Anything
  else ranks as lower case, with the letters typed upper case kept upper case; no sentence-case or
  other conventions. Acronyms: `data/en-acronyms.txt`, built by
  `tools/en-acronyms/build_acronyms.py` from Hunspell en_US (SCOWL; notice in
  `data/LICENSE-scowl.txt`).

**Cycle rows** put every mode on two cycle keys, like Gboard's あa1 key, and give Space 6u:

| Key | Cycle | From a state outside its cycle |
|---|---|---|
| **한A1** | KO → EN → symbols → KO | goes to **KO** |
| **あA1** | ひ → カ → EN → symbols → ひ | goes to **ひ** |
| **A** (Cycle, A) | goes to EN **A** from any state; if already in A, goes to **a** | |
| **a** (Cycle, a) | goes to EN **a** from any state; if already in a, goes to **A** | |
| **E** (Cycle, E) | goes to **EN predictive** (§3.3) from any state, as E does on the default row | |

- Each cycle key steps from the current state if that state is in its cycle, and otherwise jumps
  to its first state. So from JA, 한A1 goes to KO; from EN, 한A1 goes to symbols and あA1 goes to
  symbols too; from the symbol layer either key goes to its first state.
- **EN** in a cycle means the EN case last used (a on first use).
- **Symbols** in a cycle is symbol page 1. **[decide]** How to reach page 2 without an S key: a
  page key inside the layer, or a further step in the cycle?
- The A / a toggle-when-already-there rule applies to the cycle rows only. On the idempotent
  rows, A and a stay latched as in §1.1.
- **EN predictive (E)** is reachable only on the rows with an E key: both idempotent rows and
  Cycle, E. Cycle, A and Cycle, a have no E, and their cycles never pass through it. On Cycle, E,
  EN in a cycle is still EN a or A (the last used), not E.
- Stepping between ひ and カ doesn't flush (§1.1). Every other step flushes, as any mode switch does.

---

## 2. Screen class: narrow and wide

The layout depends on the **keyboard's width in dp**, not on orientation alone:

- **Wide** if the keyboard is at least `wideThresholdDp` wide (default **600 dp**). This covers
  landscape phones, an unfolded Galaxy Fold and tablets.
- **Narrow** otherwise: portrait phones and the Fold's cover screen.
- The class is checked again on `onConfigurationChanged` and `onStartInputView`. While a
  composition is in progress, a change waits until the next commit, so the layout never jumps
  mid-syllable.
- Settings: the threshold, plus a per-mode override (auto, always narrow, always wide).

| Mode | Narrow | Wide |
|---|---|---|
| EN | **EN narrow** (§3.2) | **EN wide** (§3.1) |
| KO | **KO narrow** (§4.2) | **KO wide = JIS Korean Mode A** (§4.1) |
| JA | **JA narrow = flick kana** (§4.4) | **JA wide = JIS kana** (§4.3) |

---

## 3. EN layouts

### 3.1 EN wide

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
 1  2  3  4  5  6  7  8  9  0  -  ^
 q  w  e  r  t  y  u  i  o  p  @  [
 a  s  d  f  g  h  j  k  l  ;  :  ]
 ⇪  z  x  c  v  b  n  m  ,  .  /  \
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- This is the JIS key set without the `¥` key. **Every key row has 12 keys**, so the key area is a
  straight 12 × 4 ortho grid (each key 13/12 ≈ 1.08u). The em dash (U+2014) is on symbol page 2 (§5).
- **⇪ (Caps), one grid key, left of Z.** A wide up-arrow icon. It **toggles between A and a**, the same state
  as the bottom-row A and a keys. It's highlighted while A is active.
- In A, the letter keys type capitals. The symbol and digit keys don't change.
- No one-shot Shift in EN. Case is only A or a. **[decide]** Automatic capitals at the start of a
  sentence? Off by default.

### 3.2 EN narrow

7 keys per row, stretched to fill the 13u (each key 13/7 ≈ 1.86u, about 57 dp on a 400 dp
phone, against about 36 dp for Gboard's QWERTY). There's no number row: digits are on the
symbol layer (§5).

**a (lowercase)**

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
 T  A  k  l  i  o  p
 q  w  e  r  t  y  u
 a  s  d  f  g  h  j
 z  x  c  v  b  n  m
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

**A (uppercase)**

```
 e  t  K  L  I  O  P
 Q  W  E  R  T  Y  U
 A  S  D  F  G  H  J
 Z  X  C  V  B  N  M
```

- **No case key in the grid.** Case changes only with the bottom-row **A** and **a** keys. The
  ⇪ of EN wide isn't here.
- **The first two keys of the top row type the other case**, exactly as their labels show: `T`
  and `A` in lowercase, `e` and `t` in uppercase. Each types its character and **doesn't change the
  case state**. (The likely use: capitals that start a sentence, like "The" and "A", without leaving
  lowercase, and lowercase letters inside an all-caps run without leaving uppercase.)
- The rest is QWERTY with `k l i o p` moved to the top row and `j` and `m` moved down to the ends of
  the a and z rows. All 26 letters are in both cases.

### 3.3 EN predictive (E) (draft)

The **E** key opens a mode where English is typed **by spelling on 20 big keys**. Seventeen keys are
single letters and three keys each hold three rarer letters, one tap per letter, and a language
model resolves the three shared keys. E is the one mode where predicting is the point; Principle 3
still holds, because you pick the word. It's the same in both screen classes, and all keys are taps.

A **5 × 4 straight grid, every key the same width** (13/5 = 2.6u, about 80 dp on a 400 dp phone).

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[  A  ][  E  ][  I  ][  O  ][  U  ]
[  S  ][  T  ][ ZQP ][  N  ][  H  ]
[ XJK ][  R  ][  L  ][  D  ][ VBY ]
[  G  ][  C  ][  M  ][  W  ][  F  ]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- **Each key means any letter on its label.** 17 keys are single letters; ZQP, XJK and VBY group
  the nine rarer ones. Every letter is on exactly one key, so there's never a choice of key.
- **The groups were chosen by search** (`tools/phonetic-eval/rotate_d.py`): every split of
  z q x j k v b p y into three keys of three (280 splits), scored on the same words. The first
  draft, ZQX JKV BPY, was the **worst** of the 280, because it put b and p together (but / put,
  may / map). The ten best splits are within 0.03 points of each other; ZQP XJK VBY is the best.
- **Lower case only.** E has no capitalize key and never types capitals; use A / a for them.
- **Measured** (same 20,000 words, 4.90 taps per word): first **99.5%** without context and
  **100.0%** with the previous word (rounded), in the bar **100%**. The first draft (ZQX JKV BPY)
  scored 99.2% and 99.9%. What's left is mostly single letters typed alone (v / b, k / x) and a
  few rare pairs (bob / boy, bear / year, taxes / takes).
- **Trade-off:** the letter order is new, so it has to be learned. In return the keys are 20% wider
  than the 24-key alternatives (§3.4) and it was the most accurate layout tested.

#### Decoding

- Each tap adds one key. The decoder finds every word in the word list whose letters fit the
  keys in order (each key matches any letter on its label) and ranks them with an n-gram model
  over the previous words.
- **Preedit** shows the top candidate; the bar (§6) shows the top 4. Tap a candidate to commit it.
  **[decide]** Space commits the word shown in the preedit (like T9) or commits nothing until you pick.
- Backspace removes the last tap.
- Words not in the word list can't be typed this way; use A / a. **[decide]** A spelling fallback
  inside E (for example, letter-by-letter prediction for unknown words).
- **Measured** with `tools/phonetic-eval/` (the 20,000 most frequent words in Norvig's
  Google-corpus counts, weighted by frequency; 1-gram = word frequency only, 2-gram = previous word
  as context). Scripts: `spelled_keys.py`, `rotate_d.py`.

### 3.4 EN predictive: alternatives considered

Tested on the way to §3.3 and kept for reference. None is planned; their open points apply only if
one is revived. Same tooling and word list as §3.3.

| Layout | Keys | Taps per word | First, no context | First, with previous word |
|---|---|---|---|---|
| **§3.3 (rare letters grouped, ZQP XJK VBY)** | 20 | 4.90 | **99.5%** | **100.0%** |
| A: sound keys (taps / with consonant flicks) | 24 | 4.25 / 3.73 | 89.8% | 97.4% |
| B: QWERTY groups, as drawn | 15 | 4.90 | 86–93% | 96–99% |
| C: near-exact | 24 | 4.90 | 99.0% | 99.9% |
| T9, for comparison | 8 | 4.90 | 92.9% | 99.0% |

A's figures are over the words that have a CMUdict pronunciation, so they're close to, not exactly
on, the same word list.

#### A: sound keys

For people who know how a word sounds but not how it's spelled. All keys are taps (the flick
variant is optional).

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[ あ ][ 에이 ][ 에 ][ い ][ 오 ][ う ]
[ æ  ][ 아우 ][イー][아이][o(au)][ ü ]
[p b ][ t d ][k g ][f v h][ s z ][sh ch j]
[ th ][m n ng][r l][ w y ][  *  ][  ·  ]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

A **6 × 4 straight grid, every key the same width** (13/6 ≈ 2.17u, about 67 dp on a 400 dp phone),
the same in both screen classes. All keys are taps.

##### Vowels (rows 1–2)

| Key | Sound | As in | Key | Sound | As in |
|---|---|---|---|---|---|
| あ | /ɑ/ | f**a**ther, h**o**t | æ | /æ/ | c**a**t |
| 에이 | /eɪ/ | d**ay**, m**a**ke | 아우 | /aʊ/ | n**ow**, **ou**t |
| 에 | /ɛ/ | b**e**d | イー | /iː/ | s**ee** |
| い | /ɪ/ | s**i**t | 아이 | /aɪ/ | m**y**, t**i**me |
| 오 | /oʊ/ | g**o** | o(au) | /ɔ/ | l**aw**, c**augh**t |
| う | /uː/ | f**oo**d | ü | /ʊ/ | p**u**t, b**oo**k |

- Labels are the owner's: kana and hangul where they match the English sound, Latin where they don't.
- The three row-2 slots that aren't the owner's (æ, 아우, 아이) are the a- and i-related vowels the
  first draft didn't have. *(open if revived)* 아우 or /ʌ/ (c**u**p) in its slot: in testing the two were
  0.1 points apart. Whichever is left out is typed as two keys (아우 = あ + う) or as あ (/ʌ/).
- /ɔɪ/ (b**oy**) is typed o(au) + い. The "er" sound (b**ir**d) is typed on r l.
- **Unstressed vowels** (the /ə/ in **a**bout, penc**i**l) are hard to hear, so the decoder accepts
  **any vowel key** for them. *(open if revived)* **ü** is /ʊ/ and **う** is /uː/ (assumed).

##### Consonants and special keys (rows 3–4)

Ten sound keys. Voiced and voiceless pairs share a key, as do the sounds a Korean or Japanese
ear merges.

| Key | Sounds | Key | Sounds |
|---|---|---|---|
| p b | /p b/ | th | /θ ð/ (**th**in, **th**is) |
| t d | /t d/ | m n ng | /m n ŋ/ |
| k g | /k g/ | r l | /r l/ and /ɝ/ (b**ir**d) |
| f v h | /f v h/ | w y | /w j/ |
| s z | /s z/ | * | **wildcard**: any one sound |
| sh ch j | /ʃ ʒ tʃ dʒ/ | · | **end of word part** |

- **\*** matches any one sound, vowel or consonant, for a sound you're not sure of.
- **·** marks a boundary inside a word, so long words decode as parts: inter · ballistic. The
  decoder matches each part separately and joins them without a space.

##### Decoding by sound

- Each tap adds one sound. The decoder looks up every word whose pronunciation matches the
  sequence (pronunciation dictionary, e.g. CMUdict), and ranks them with an n-gram model over
  the previous words.
- **Preedit** shows the top candidate; the bar (§6) shows the top 4. Tap a candidate to commit it.
  *(open if revived)* Space commits the word shown in the preedit (like T9) or commits nothing until you pick.
- Backspace removes the last sound.
- **Measured** (offline, on the 20,000 most frequent English words in Norvig's Google-corpus counts
  that have a CMUdict pronunciation, weighted by frequency): with the previous word as context, the
  intended word is **first about 97–98%** of the time and **in the 4-candidate bar about 100%**.
  Without context, first about 90–93%, in the bar 99.6–99.8%. Most misses are true homophones
  (to / too / two / do), which context resolves. Scripts: `tools/phonetic-eval/`.
- Words not in the pronunciation dictionary can't be typed this way; use A / a. *(open if revived)* A
  spelling fallback inside E.

##### Variant: consonant flicks

Same grid and keys; the ten consonant
keys also flick, as KO narrow's consonants do (§4.2):

| Consonant key | C (tap) | E | W | N | S |
|---|---|---|---|---|---|
| any of the ten | the consonant alone | + あ /ɑ/ | + ʌ (uh, the sound nearest 어) | + o(au) /ɔ/ | + う /uː/ |

- A flick is the consonant **followed by that vowel**, in one gesture: t d flicked S is /tuː/
  (two, do), k g flicked W is /kʌ/ (cup, come).
- **ʌ has no key of its own**; a flick is the only way to type it, and only after a consonant. On
  its own (**u**p, **o**ther) it's typed あ, as in the tap-only layout. The decoder accepts the W
  flick for unstressed /ə/ too, like any vowel key.
- The vowel keys, \* and · don't flick. Flick detection is as in §4.2.
- *(open if revived)* "au" on N is read as o(au) /ɔ/, the key's label. 아우 /aʊ/ there tested almost the same.
- **Measured** (`tools/phonetic-eval/sound_keys.py`, same 20,000 words): taps only, **4.25** taps
  per word; with consonant flicks, **3.73** gestures per word (12% fewer), at the same accuracy
  (first 97.4% with context, in the bar ~100%). For comparison the words average 4.93 letters.
  What it costs, a flick per syllable instead of a tap, needs a hands-on test.

#### B: QWERTY groups

Spelling-based, for people who know the spelling and the QWERTY layout. One tap per letter.

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[ A ][ E ][ I ][ O ][ U ][ Y ]
[   QWER   ][   RTYU   ][   UIOP   ]
[   ASDF   ][   FGHJ   ][   HJKL   ]
[   ZXCV   ][   CVB    ][   VBNM   ]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- **15 keys.** Row 1 has 6 keys (13/6 ≈ 2.17u); rows 2–4 have 3 keys each (13/3 ≈ 4.33u), and
  each lower key spans exactly two top-row keys, so the columns line up.
- **Every key means any letter on its label.** The vowels on row 1 are exact; each lower key is
  an overlapping chunk of its QWERTY row. R, U, F, H, J, C, V and B are on two keys, and either
  one is correct.
- The decoder is the one in §3.3. Backspace removes the last tap. 
- **Measured** (`tools/phonetic-eval/spelled_keys.py`, the 20,000 most frequent words in Norvig's
  counts, frequency-weighted; 4.90 taps per word, one per letter):

  | Variant | First, no context | First, with previous word | In the bar |
  |---|---|---|---|
  | As drawn, overlaps pressed on the left key | 93.0% | 98.8% | 100.0% |
  | As drawn, overlaps pressed on the right key | 86.1% | 96.2% | 99.9% |
  | As drawn, overlaps pressed on either key at random | 89.1% | 97.5% | 100.0% |
  | **Vowel-free lower keys** (QWR RT P · SDF FGHJ HJKL · ZXCV CVB VBNM) | **94.3%** | **99.0%** | 100.0% |
  | T9, for comparison (8 keys) | 92.9% | 99.0% | 100.0% |

- **The vowels on the lower keys cost the most.** Pressing QWER for w also allows e, so many
  more words match. With the lower keys made consonant-only (vowels only from row 1), accuracy
  rises and it **stops mattering which key you press for R, U, F, H, J, C, V or B**.
  *(open if revived)* Keep the vowels on the lower keys as drawn, or make them consonant-only (labels
  could show the vowels dimmed).

#### C: near-exact

Spelling-based, 24 equal keys (6 × 4, ≈ 2.17u each). Almost every key is one letter; only four
keys are pairs, so the decoder has very little to resolve.

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[ A ][ E ][ I ][ O ][ U ][ Y ]
[ Q ][ W ][ R ][ T ][ P ][ ⇪ ]
[ S ][ D ][ F ][GH ][HJ ][KL ]
[ Z ][ X ][ C ][ V ][ B ][NM ]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- **Each key means any letter on its label.** GH, HJ, KL and NM are pairs; H is on two keys, and
  either is correct. All 26 letters are present.
- **⇪ capitalizes the previous word.** It isn't Caps Lock or Shift: it changes the word just
  before the cursor (the one being typed, or the last one committed if nothing is being typed)
  to start with a capital, so you type the word first and capitalize it afterwards. "hello" ⇪ →
  "Hello". *(open if revived)* A second ⇪ right away: ALL CAPS, then back to lower case?
- The decoder is the one in §3.3. Backspace removes the last tap.
- **Measured** (`tools/phonetic-eval/spelled_keys.py`, same 20,000 words, 4.90 taps per word):
  first **99.0%** without context and **99.9%** with the previous word, in the bar **100%**. It
  makes no difference which key is pressed for H. The remaining collisions are mostly n / m
  (am / an, then / them, might / night) and k / l (took / tool, make / male).
- **Trade-off:** the most accurate of the three and the closest to plain typing,
  but the keys are smaller (≈ 2.17u, like A) and you must know the spelling.

---

## 4. KO and JA layouts

### 4.1 KO wide: JIS Korean Mode A

The full **JIS Korean Mode A** key map (`jis-korean/docs/spec.md` §3) on the same grid as EN wide:

```
 ㅃ ㅉ ㄸ ㄲ ㅆ ㅟ ㅝ ㅚ ㅢ ㅘ  !  (
 ㅂ ㅈ ㄷ ㄱ ㅅ ㅛ ㅕ ㅑ ㅐ ㅔ  '  [
 ㅁ ㄴ ㅇ ㄹ ㅎ ㅗ ㅓ ㅏ ㅣ  ~  :  ]
 ⇧  ㅋ ㅌ ㅊ ㅍ ㅠ ㅜ ㅡ  ,  .  ·  \
```

- **12 keys per row, like EN wide (§3.1).** Mode A's 13th number-row key (`—`, Shift `^`) is
  left out so the key area is a straight 12 × 4 grid. Both characters stay reachable on the symbol
  layer (§5). The engine is unchanged; the shell just never sends that key.
- **⇧ (Shift), one grid key, left of Z.** A one-shot modifier. Tap it once for the next key only, double-tap
  to lock (shown as locked). It sends `JK_MOD_SHIFT` with the next key.
  Mode A puts **digits on Shift** (⇧ then 1 gives 1), and `?` `)` `"` `「` `-` `;` `」` `_` there too.
- Keys are sent to the jis-korean engine as physical key ids (`JK_KEY_*`). The engine does
  composition, the implied ㅇ, backspace and commit. Nothing here changes what it does.
- **Shift+Z is not F10.** The engine treats Shift+Z as F10 (half-width letters), but uim never
  sends it: ⇧ then ㅋ types plain ㅋ (the shell sends `KEYZ` without Shift, and the one-shot ⇧ is
  used up). The literal letters are still one tap away, because the candidate bar's "English on
  the same keys" reading (§6) shows them. The engine itself is unchanged.
- **F6–F10 / Hanja.** There are no function-row keys. Hanja and the other readings show up in the
  candidate bar (§6) from the engine's predictor. **[decide]** Do we need a 한자 key?

### 4.2 KO narrow

An **8 × 4 straight grid, every key the same width** (13/8 ≈ 1.63u, about 50 dp on a 400 dp
phone). The left five columns are the consonants; the right three and the ㅋ row's fifth slot are
vowel keys. Every key is tapped or **flicked N / E / S / W**.

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[ㅃ][ㅉ][ㄸ][ㄲ][ㅆ][ㅢ][ㅙ][ㅞ]
[ㅂ][ㅈ][ㄷ][ㄱ][ㅅ][ㅚ][ㅟ][ㅣ]
[ㅁ][ㄴ][ㅇ][ㄹ][ㅎ][ㅡ][ㅏ][ㅓ]
[ㅋ][ㅌ][ㅊ][ㅍ][ㅘ][ # ][ㅒ][ㅖ]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

Legends show the tap (C). A key's flick outputs are drawn small on the matching side.

#### Flick directions

- **C** is a tap. **N / E / S / W** is a flick up / right / down / left: the touch goes down on the
  key and moves at least the **flick threshold** before it lifts. Default threshold: 40% of the key
  width **[decide]**. The direction is the axis the movement is closest to (no diagonals).
- The key is the one under the **touch-down** point (§1 hit-testing). Where the finger lifts doesn't matter.
- A direction marked — types nothing (light haptic). **[decide]** Fall back to C instead?
- The convention follows flick-korean's dot: **N ㅗ, E ㅏ, S ㅜ, W ㅓ** (the vowel goes where its
  dot points). The ㅘ key and every consonant key use exactly these four.

#### Consonant keys

| Key | C | N | E | S | W |
|---|---|---|---|---|---|
| any consonant *X* | *X* | *X* + ㅗ | *X* + ㅏ | *X* + ㅜ | *X* + ㅓ |

- **A tap is the consonant alone.** A flick is the consonant **plus the ㅘ key's vowel for that
  direction**: ㄱ flicked E is 가, ㅂ flicked N is 보, ㅁ flicked W is 머. Unlike the ㅘ key, a
  consonant has no ㅘ on its tap.
- So a common CV syllable is one gesture, and every other vowel still comes from the vowel keys.

#### Vowel keys

| Key (C) | Position | C | N | E | S | W |
|---|---|---|---|---|---|---|
| **ㅘ** | row 4, col 5 (right of ㅍ) | ㅘ | ㅗ | ㅏ | ㅜ | ㅓ |
| **ㅢ** | row 1, col 6 | ㅢ | ㅡ | ㅣ | ㅡ | ㅣ |
| **ㅙ** | row 1, col 7 | ㅙ | ㅗ | ㅐ | ㅜ | — |
| **ㅞ** | row 1, col 8 | ㅞ | ㅗ | ㅔ | ㅜ | — |
| **ㅚ** | row 2, col 6 | ㅚ | ㅗ | ㅣ | ㅟ | — |
| **ㅟ** | row 2, col 7 | ㅟ | ㅚ | ㅣ | ㅜ | — |
| **ㅣ** | row 2, col 8 | ㅣ | ㅡ | ㅏ | ㅡ | ㅓ |
| **ㅡ** | row 3, col 6 | ㅡ | ㅗ | ㅣ | ㅜ | ㅣ |
| **ㅏ** | row 3, col 7 | ㅏ | ㅘ | ㅐ | — | ㅐ |
| **ㅓ** | row 3, col 8 | ㅓ | — | ㅔ | ㅝ | ㅔ |
| **#** | row 4, col 6 | `#` **[decide]** | ㅛ | ㅑ | ㅠ | ㅕ |
| **ㅒ** | row 4, col 7 | ㅒ | — | — | — | — |
| **ㅖ** | row 4, col 8 | ㅖ | — | — | — | — |

- **#** is flick-korean's two-dot key: the y-vowels in the ㅘ key's directions (ㅛ where ㅗ is,
  ㅑ where ㅏ is, and so on). Its tap types `#` as in flick-korean; **[decide]** whether it should
  type nothing instead, since symbols live on the S layer.
- **ㅒ and ㅖ** are tap-only keys.
- **Coverage:** all 21 vowels are one gesture (a tap or one flick), and every 이중모음
  (ㅑ ㅒ ㅕ ㅖ ㅘ ㅙ ㅛ ㅝ ㅞ ㅠ ㅢ) has its own designation. 12 vowels are taps (the keys' C legends); ㅗ ㅜ ㅐ ㅔ ㅛ ㅑ ㅠ ㅕ ㅝ need a flick.
  Several vowels appear on more than one key on purpose, so the nearest key usually has it.
- The duplicate placements are the design as given; **[decide]** whether to keep them all once
  the layout has been used.

#### Engine mapping

KO narrow is still a **subset view of Mode A**: every output is sent to the same jis-korean
engine as Mode A physical keys, so there are no new composition rules.

| Output | Physical keys sent |
|---|---|
| ㅃ ㅉ ㄸ ㄲ ㅆ | `DIGIT1`–`DIGIT5` |
| ㅂ ㅈ ㄷ ㄱ ㅅ | `KEYQ`–`KEYT` |
| ㅁ ㄴ ㅇ ㄹ ㅎ | `KEYA`–`KEYG` |
| ㅋ ㅌ ㅊ ㅍ | `KEYZ`–`KEYV` |
| ㅛ ㅕ ㅑ ㅐ ㅔ | `KEYY` `KEYU` `KEYI` `KEYO` `KEYP` |
| ㅒ ㅖ | Shift + `KEYO`, Shift + `KEYP` |
| ㅗ ㅓ ㅏ ㅣ | `KEYH` `KEYJ` `KEYK` `KEYL` |
| ㅠ ㅜ ㅡ | `KEYB` `KEYN` `KEYM` |
| ㅟ ㅝ ㅚ ㅢ ㅘ | `DIGIT6`–`DIGIT0` |
| ㅙ | `KEYH` then `KEYO` (ㅗ + ㅐ; Mode A has no ㅙ key) |
| ㅞ | `KEYN` then `KEYP` (ㅜ + ㅔ) |
| consonant flick | the consonant's key, then the vowel's key |

- Because these are ordinary Mode A keys, a vowel can still combine with the vowel before it
  (고 then ㅏ → 과), exactly as on KO wide. ㅙ and ㅞ are sent as a pair inside one gesture, so
  they always form one medial.
- **Backspace follows the JIS Korean automaton** (`jis-korean/docs/spec.md` §4.3), not the
  gesture: the **unfinished** syllable being composed loses its last jamo and recomposes
  (간 → 가 → ㄱ → empty), and a **finished** syllable is deleted as a whole block. A flick is
  two or three engine key events, but that doesn't change the rule: 가 from one flick, still being
  composed, backspaces to ㄱ.

### 4.3 JA wide: JIS kana (JIS X 6002)

```
 ぬ ふ あ う え お や ゆ よ わ ほ へ ー
 た て い す か ん な に ら せ ゛ ゜  Hi
 ち と し は き く ま の り れ け む  Ka
 ⇧  っ つ さ そ ひ こ み も ね る め ろ
```

- **13 keys in every row**, so the key area is a straight 13 × 4 grid (each key 1u). The three
  keys that make it square are Hi, Ka and っ.
- **Hi, end of the た row, and Ka, end of the ち row,** are the script keys, like ⇪ in EN wide
  (§3.1). **They do exactly what the bottom row's ひ and カ do** (§1.1): Hi switches to hiragana
  and Ka to katakana, without flushing. The active one is highlighted. They don't convert the
  segment the way F6 and F7 do in PC IMEs; the candidate bar offers both scripts instead (§6).
- **っ, between ⇧ and つ.** Types っ with one tap. ⇧ then つ (JIS Shift+Z) still types っ too.

- **⇧ (Shift), 1u, left of っ.** One-shot or locked, as in §4.1. Shifted keys (standard JIS kana):
  ぁ(3) ぃ(E) ぅ(4) ぇ(5) ぉ(6) ゃ(7) ゅ(8) ょ(9) を(0) っ(Z) 「([) 」(]) 、(,) 。(.) ・(/).
- **゛ and ゜** apply to the kana before them in the composition (か゛→が, は゜→ぱ, う゛→ゔ). After a kana
  they can't apply to, they're inserted as is.
- **Kana and katakana.** The kana keys type the active script (ひ or カ, §1.1), which then goes to
  conversion (§6).

### 4.4 JA narrow: flick kana

A **5 × 4 straight grid, every key the same width** (13/5 = 2.6u, about 80 dp on a 400 dp
phone). Flicks work as in §4.2 (touch-down picks the key, threshold, no diagonals) and follow
**Gboard's 12-key convention**: **C a, W i, N u, E e, S o**.

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
[あ][か][さ][ぁ][、]
[た][な][は][っ][。]
[ま][や][ら][ゃ][「」]
[゛][わ][゜][ゎ][・]
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- **Columns 1–3 are Gboard's block** in Gboard's positions: あかさ / たなは / まやら, with わ
  under や. ゛ and ゜ flank わ, where Gboard has its 小゛゜ key and its punctuation key.
- **Column 4 is small kana**, each in the row of its full-size kana.
- **Column 5 is punctuation**, always the rightmost column.

| Key | C | W | N | E | S |
|---|---|---|---|---|---|
| あ か さ た な は ま ら | *-a* | *-i* | *-u* | *-e* | *-o* |
| や | や | （ | ゆ | ） | よ |
| わ | わ | を | ん | ー | — |
| ぁ (小) | ぁ | ぃ | ぅ | ぇ | ぉ |
| っ (小) | っ | — | — | — | — |
| ゃ (小) | ゃ | — | ゅ | — | ょ |
| ゎ (小) | ゎ | — | — | — | — |
| ゛ | ゛ | — | — | — | — |
| ゜ | ゜ | — | — | — | — |
| 、 | 、 | ！ | — | ？ | — |
| 。 | 。 | ！ | — | ？ | — |
| 「」 | `"` | 「 | 『 | 」 | 』 |
| ・ | ・ | caret | — | caret | — |

- **や and わ** keep Gboard's extras: （ ） on や W/E, and を W, ん N, ー E on わ.
- **゛ and ゜** apply to the kana before them, as on JA wide (§4.3): か゛→が, は゜→ぱ, う゛→ゔ.
  So all 21 dakuten and 5 handakuten kana come from two keys.
- **、 and 。** both flick **W for ！ and E for ？**, so either key gives the sentence enders.
- **「」** taps `"`. W and E are 「 」, and N and S are the doubled 『 』. **[decide]** ASCII `"`
  (U+0022) or full-width ＂ (U+FF02)?
- **・** taps ・. A **horizontal drag moves the caret** instead of typing: one character per step,
  for as long as the finger stays down. It flushes the composition first. **[decide]** The step
  size (default: half the flick threshold per character).
- **Coverage:** all 46 basic kana, all 10 small kana, the 26 voiced kana via ゛ ゜, and ー.
- ⇧, Hi and Ka aren't on this layout. Small kana have their own keys, so ⇧ isn't needed, and the
  script is chosen with the bottom row's ひ and カ (§1.1), whose legends and outputs switch the
  kana keys between hiragana and katakana.

---

## 5. Symbol layer (S)

**One layer for every mode and both screen classes.** It's a **10 × 4** grid (each key 1.3u)
with the **digits on the top row**. The top bar and the bottom row are unchanged.

**JIS conventions.** Symbols follow the JIS keyboard, not US ANSI: the shifted digits are
`! " # $ % & ' ( )` (not `! @ # $`), and the other symbols come in JIS key order, each
unshifted symbol next to its Shift partner (`^ ~`, `[ {`, `] }`, `; +`, `: *`, `/ ?`, `\ _`
and so on).

```
[Copy][Paste][     Candidates 8u     ][  Backspace 3u  ]
 1  2  3  4  5  6  7  8  9  0
 !  "  #  $  %  &  '  (  )  =
 -  ^  ~  |  @  `  [  {  ]  }
 ;  +  :  *  ,  .  /  ?  \  _
[S][E][A][a][  Space 3u  ][한][ひ][カ][  Enter 3u  ]
```

- **Row 2** is JIS Shift+1 to Shift+9, then `=` (JIS Shift+−, since JIS Shift+0 types nothing).
- **Rows 3–4** are the remaining JIS symbol keys in order. Only `< >` (Shift+, and Shift+.) don't
  fit, so they go to page 2.
- **Rows 2–4 are a proposal [decide].** "10 × 4, digits on top, JIS conventions" is decided.
- **Page 2 [decide]:** `< > — …` and the CJK punctuation `、 。 ・ 「 」 『 』 【 】 ※ 〜 ￥`, under
  the same digit row. Reached by tapping S again on page 1. A third tap closes the layer.
- Tap S to open it. Any mode key (E, A, a, 한, ひ, カ) also leaves the layer. A symbol tap doesn't leave it.
- In KO and JA, the composition is committed first, then the symbol is inserted.

---

## 6. Candidate bar

| Mode | Source | Content |
|---|---|---|
| KO (both classes) | jis-korean predictor (full build, `learned.bin`) | Its up to 4 readings: the Korean as composed, English on the same keys, finals-kept, Hanja, particles (jis-korean spec §10.2) |
| EN | uim word list plus a user dictionary on the device | Up to 4 completions of the current word |
| JA | Kana-kanji converter (§8 M6): a Viterbi search over Mozc's open dictionary (`data/kanji.bin`, built by `tools/kanji/build_kanji.py`, ported from jis-moonshot) | Conversions of the whole composition, cheapest first, then the other script in the last slot. The kana as typed isn't listed (it's the preedit, and Space commits it). In カ, all-katakana conversions come first and hiragana is last; in ひ, katakana is last. Space moves to the next candidate (IME convention) **[decide]**; for now Space commits the kana as typed |

- The first candidate is never committed automatically. Space commits what you typed, and a candidate only goes in
  when you tap it. (The JA Space-to-convert behavior is a deliberate exception and needs a decision.)
- If there are more than 4 candidates, a `▾` at the right end of the bar opens a full candidate
  sheet over the key area.
- Learning from picks is opt-in, stays on the device and can be reset (jis-korean §10.11
  learning in KO).

---

## 7. Architecture (Android)

The repo follows the pattern of `flick-korean` and `jis-korean`: a Rust engine with one C ABI and thin shells.

```
uim-mobile/
  engine/                 Rust crate `uim-engine`, C ABI `uim_*` (include/uim.h, src/ffi.rs)
    deps: jis-korean-engine (path ../jis-korean/engine for development, pinned git rev for CI)
    src/mode.rs           Mode state machine: EN-a / EN-A / EN-P (predictive) / KO / JA-ひ / JA-カ / S, flush on switch (not ひ ↔ カ)
    src/latin.rs          EN: direct commit, word completion
    src/kana.rs           JA: kana composition, ゛ ゜, small kana
    src/convert/          JA kana-kanji converter (M6)
    src/layout.rs         Layout data → key ids, labels, flick outputs, slot geometry (in u), reserved slots
    tests/header.rs       Fails if include/uim.h and src/ffi.rs drift
  android/
    app/src/main/cpp/uim_jni.c       JNI over the C ABI
    app/src/main/java/me/sjun/uim/
      UimInputService.kt   InputMethodService: composing text, commits, editor actions
      KeyboardView.kt      Custom Canvas View (not Compose: input latency and IME lifecycle)
      Geometry.kt          Grid → rectangles; `hit()` is the only place a touch becomes a key
                           (and, in KO and JA narrow, a key plus a flick direction or a caret drag)
      ScreenClass.kt       Narrow or wide decision with hysteresis (§2)
      SettingsActivity.kt  Compose UI: enable IME, threshold, row height, bottom row (§1.2), haptics, learning
  web/                    (later) wasm playground with the same engine
```

- **Engine API:** event in, actions out. A flick is part of the key id (key + direction from the
  layout data), so the shell never decides what a flick types.
  `uim_key(engine, key_id, mods) → { commit, preedit,
  candidates[], action }`. No callbacks and no global state. One engine per input context.
  The engine must never panic.
- **KO delegates fully** to `jis-korean-engine`. uim never re-implements Hangul composition.
- **Toolchain** (same as flick-korean): JDK 21, compileSdk/targetSdk 36, minSdk 29,
  NDK 30.0.16248370, CMake 4.1.2. The engine is built as a staticlib per ABI (`aarch64`, `armv7`,
  `x86_64`) by a Gradle task that runs `cargo rustc --crate-type staticlib`.
- **Package:** `me.sjun.uim`. The app is named "uim".
- **Size budget:** under 25 MB APK, covering jis-korean predict data (~3 MB), hanja (~4 MB) and the JA dictionary
  (to be measured in M6).

---

## 8. Build plan (Android)

Each milestone ends on a device (the user's phone and a Galaxy Fold, or an emulator with the Fold profile).

| # | Milestone | Scope | Done when |
|---|---|---|---|
| **M0** | Skeleton | Repo, Gradle with the Rust build, JNI, an IME that shows up in system settings. `KeyboardView` draws the 13u × 6-row grid from layout data. Top bar and bottom row: Copy, Paste, Backspace (with repeat), Space, Enter (editor actions). | Space, Enter and Backspace work in any app. Copy and Paste work. |
| **M1** | EN and symbols | EN wide (§3.1) and EN narrow (§3.2), basic narrow/wide switching by width (§2), A/a/⇪ case state, the other-case keys in narrow, page 1 of the symbol layer (§5, so digits work on narrow), the E key and its empty reserved area (§3.3), mode-key highlighting, haptics. Hit-testing with no dead zones (§1). | Can type an English paragraph with digits on both a phone and an unfolded Fold. Unit tests for case state, plus screenshot tests of both layouts and the symbol layer. |
| **M2** | KO wide | Link jis-korean-engine. Mode A layout (§4.1), ⇧ one-shot or lock, preedit as composing text, flush on mode switch, focus loss and app switch. | jis-korean's own engine tests pass via `uim-engine`. Typing on the Fold inner screen matches JIS Korean on a Mac. |
| **M3** | KO narrow | The §4.2 8 × 4 layout: flick detection (N/E/S/W, threshold) in `Geometry`, consonant flicks, the vowel keys and the Mode A mapping, including ㅙ and ㅞ as key pairs. Screen-class hysteresis (no switch mid-composition, §2) and settings overrides. | Can type a Korean paragraph with every vowel. Folding and unfolding swaps narrow and wide without losing text. |
| **M4** | KO candidates | jis-korean predictor (full) in the candidate bar, the `▾` sheet, opt-in learning. | The 4 readings match jis-korean on the same keys. |
| **M5** | JA kana | JIS kana layout (§4.3), ⇧ small kana, ゛ ゜, hiragana commit (no conversion yet). JA narrow flick layout (§4.4), including ・ caret drag. | Can type a kana sentence on both a phone and an unfolded Fold. |
| **M6** | JA conversion | **Built (2026-11-02):** option (b), jis-moonshot's converter and data (70% exact sentences on Mozc's evaluation set, 5.6 MB compressed in the APK, about 0.1 ms per keystroke). Whole-composition conversion only; segment editing is still to do. **Spike first (1–2 days):** (a) Mozc via NDK vs (b) a small Rust Viterbi converter over Mozc's OSS dictionary data. Check license, size and quality for each. Default pick: (b), so everything stays in one Rust engine and can target wasm. Then: segment conversion, candidates, Space to convert **[decide]**. | Common sentences convert correctly. Size is within budget. |
| **M7** | Symbols page 2 and EN prediction | Symbol page 2 and the final rows 2–4 (§5) once decided. EN completions and a user dictionary. The E layout (§3.3): word list, key-sequence decoder and n-gram ranking. | The `tools/phonetic-eval` numbers hold on the shipped data. |
| **M8** | Accuracy work | **Fit to your hand:** opt-in calibration (type a fixed text, measure the average touch offset per row and key, apply it as a fixed shift, so taps stay deterministic). Opt-in local touch log for tuning. Measure WPM, error rate and backspace rate against Gboard. | Fewer errors than the Gboard baseline on the same phone, by the user's own measurement. |
| **M9** | Daily use | Run it as the daily keyboard for 2 weeks. Fix what turns up. Settings polish, light and dark themes. | |

M3 depends on M2, and M6 on M5. M5 can start alongside M3 and M4 if needed.

**Tests:** `cargo test` on the engine (composition, mode switching, kana ゛ ゜, the C ABI header check);
JVM unit tests for `Geometry.hit()` and `ScreenClass`; screenshot tests (Roborazzi) for every layout
in both screen classes; on-device instrumented tests for composing text and editor actions.

---

## 9. Open questions

1. **JA narrow** (§4.4): the 「」 tap character and the caret-drag step.
2. **KO narrow flicks** (§4.2): the threshold, what an empty direction does, the # key's tap,
   and whether to keep every duplicate vowel.
3. **EN narrow's other-case keys** (§3.2): confirm that `T A` / `e t` type their character without
   changing the case state, as written.
4. Grid filling: stretch or JIS stagger (§1).
5. Copy with no selection (§1.1).
6. A 한자 key in KO wide (§4.1)?
7. JA: Space to convert or Space to commit.
8. Symbol layer rows 2–4 and page 2 (§5).
9. Automatic capitals in EN (§3.1, §3.2).
10. **EN predictive (E)** (§3.3): Space behavior, a spelling fallback for unknown words, and the licence of the word-frequency data.
11. iOS / iPhone: out of scope for now. The engine's C ABI keeps that door open (XCFramework, as
   flick-korean plans).
12. **Cycle bottom rows** (§1.2): reaching symbol page 2.
