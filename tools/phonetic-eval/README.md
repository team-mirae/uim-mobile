# phonetic-eval

Offline checks for the EN predictive sound-key layout (SPEC.md §3.3).

Get the data (not checked in):

```
mkdir -p data
curl -sSfL -o data/count_1w.txt https://norvig.com/ngrams/count_1w.txt
curl -sSfL -o data/count_2w.txt https://norvig.com/ngrams/count_2w.txt
curl -sSfL -o data/cmudict.dict https://raw.githubusercontent.com/cmusphinx/cmudict/master/cmudict.dict
```

Run:

```
python3 sound_keys.py                               # candidate A (sound keys): taps vs consonant flicks
python3 sound_keys.py variants                      # candidate A: vowel-slot variants
python3 letter_groups.py data/count_1w.txt 20000    # spelling-based letter groups, T9 and QWERTY for comparison
python3 spelled_keys.py                             # candidates B, C, D and variants, 1-gram and 2-gram
python3 rotate_d.py                                 # candidate D: all 280 splits of the nine grouped letters
```

`sound_keys.py` takes the 20,000 most frequent words that have a CMUdict pronunciation, types each
by its first pronunciation, and ranks every word that matches the same key sequence (by unigram
count, then by bigram count after the previous word, with stupid backoff). The layout and the
variants compared are at the top and bottom of the file.
