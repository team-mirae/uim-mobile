/* uim engine C ABI (src/ffi.rs). Strings are UTF-8, not NUL-terminated: read ptr + len. */
#ifndef UIM_H
#define UIM_H
#include <stddef.h>
#include <stdint.h>

typedef struct UimHandle UimHandle;
typedef struct UimKanji UimKanji;

enum { UIM_ACTION_NONE = 0, UIM_ACTION_DELETE = 1, UIM_ACTION_ENTER = 2, UIM_ACTION_COPY = 3,
       UIM_ACTION_PASTE = 4, UIM_ACTION_CURSOR = 5 };

UimHandle *uim_new(void);
void uim_free(UimHandle *h);
void uim_set_wide(UimHandle *h, uint32_t wide);
void uim_set_bottom(UimHandle *h, uint32_t bottom);
void uim_load_words(UimHandle *h, const uint8_t *ptr, size_t len);
void uim_load_bigrams(UimHandle *h, const uint8_t *ptr, size_t len);
/* After uim_load_words: data/en-acronyms.txt (all caps ranks these first). */
void uim_load_acronyms(UimHandle *h, const uint8_t *ptr, size_t len);
/* JA conversion data (data/kanji.bin): parse once, off the UI thread (NULL if malformed), give it
 * to engines, then free the host's reference. Engines keep their own. */
const UimKanji *uim_kanji_new(const uint8_t *ptr, size_t len);
void uim_kanji_free(const UimKanji *k);
void uim_set_kanji(UimHandle *h, const UimKanji *k);
/* dir: 0 tap, 1 N, 2 E, 3 S, 4 W */
void uim_press(UimHandle *h, uint32_t row, uint32_t col, uint32_t dir);
/* A key with the HOLD flag went down (1) or up (0). The shell sends its tap separately, and only
 * if no other key was pressed while it was down. */
void uim_hold(UimHandle *h, uint32_t on);
void uim_drag(UimHandle *h, int32_t steps);
void uim_candidate(UimHandle *h, uint32_t index);
void uim_flush(UimHandle *h);
const uint8_t *uim_commit_ptr(UimHandle *h);
size_t uim_commit_len(UimHandle *h);
const uint8_t *uim_preedit_ptr(UimHandle *h);
size_t uim_preedit_len(UimHandle *h);
const uint8_t *uim_layout_ptr(UimHandle *h);
size_t uim_layout_len(UimHandle *h);
uint32_t uim_action(UimHandle *h);
int32_t uim_action_arg(UimHandle *h);
uint32_t uim_candidate_count(UimHandle *h);
const uint8_t *uim_candidate_ptr(UimHandle *h, uint32_t i);
size_t uim_candidate_len(UimHandle *h, uint32_t i);

#endif
