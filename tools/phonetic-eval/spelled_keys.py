"""Spelling-based ambiguous keys (each key = a set of letters) with 1-gram and 2-gram ranking,
comparable with sound_keys.py. Usage: python3 spelled_keys.py"""
import os, re, sys, collections, itertools
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data") + "/"
N = 20000
uni = {}
for line in open(D + "count_1w.txt"):
    w, c = line.split("\t")
    if re.fullmatch(r"[a-z]+", w): uni[w] = int(c)
vocab = [w for w, _ in sorted(uni.items(), key=lambda t: -t[1])[:N]]
V = set(vocab); tot1 = sum(uni[w] for w in vocab)
big = {}
for line in open(D + "count_2w.txt"):
    ab, c = line.rsplit("\t", 1); p = ab.split(" ")
    if len(p) == 2 and p[1] in V and p[0] in uni: big[(p[0], p[1])] = int(c)

trie = {}
for w in vocab:
    n = trie
    for ch in w: n = n.setdefault(ch, {})
    n["$"] = w

def evaluate(name, keys, choose):
    """keys: list of letter sets; choose(word, i) -> index of the key the user presses for word[i]."""
    def seq(w): return tuple(choose(w, i) for i in range(len(w)))
    cache = {}
    def cands(w):
        s = seq(w)
        if s not in cache:
            out = []
            def go(n, j):
                if j == len(s):
                    if "$" in n: out.append(n["$"])
                    return
                for ch in keys[s[j]]:
                    if ch in n: go(n[ch], j + 1)
            go(trie, 0); cache[s] = out
        return cache[s]
    r1 = r4 = 0
    for w in vocab:
        c = sorted(cands(w), key=lambda x: -uni[x]); r = c.index(w) + 1
        if r == 1: r1 += uni[w]
        if r <= 4: r4 += uni[w]
    b1 = b4 = bt = 0
    for (a, b), c in big.items():
        cs = cands(b)
        sc = lambda x: big[(a, x)] / uni[a] if (a, x) in big else 0.4 * uni[x] / tot1
        r = sorted(cs, key=lambda x: -sc(x)).index(b) + 1
        bt += c
        if r == 1: b1 += c
        if r <= 4: b4 += c
    L = sum(uni[w] * len(w) for w in vocab) / tot1
    print(f"{name:46s} keys {len(keys):2d}  taps/word {L:4.2f} | 1-gram: first {100*r1/tot1:5.1f}%  bar {100*r4/tot1:5.1f}% | 2-gram: first {100*b1/bt:5.1f}%  bar {100*b4/bt:5.1f}%")

def first_key(keys):
    return lambda w, i: next(k for k, s in enumerate(keys) if w[i] in s)
def last_key(keys):
    return lambda w, i: max(k for k, s in enumerate(keys) if w[i] in s)

if __name__ == "__main__":
    QG = ["a", "e", "i", "o", "u", "y", "qwer", "rtyu", "uiop", "asdf", "fghj", "hjkl", "zxcv", "cvb", "vbnm"]
    print("vocab: top", N, "words")
    evaluate("QWERTY groups, overlaps on the left key", QG, first_key(QG))
    evaluate("QWERTY groups, overlaps on the right key", QG, last_key(QG))
    # vowels typed on their QWERTY-row group instead of the top row (worst case)
    vow_on_row = lambda w, i: (max if w[i] in "aeiouy" else min)(k for k, s in enumerate(QG) if w[i] in s)
    evaluate("QWERTY groups, vowels typed on the letter rows", QG, vow_on_row)
    C = ["a", "e", "i", "o", "u", "y", "q", "w", "r", "t", "p", "s", "d", "f", "gh", "hj", "kl", "z", "x", "c", "v", "b", "nm"]
    evaluate("C: near-exact, h pressed on GH", C, first_key(C))
    evaluate("C: near-exact, h pressed on HJ", C, last_key(C))
    Dk = ["a", "e", "i", "o", "u", "s", "t", "zqx", "n", "h", "jkv", "r", "l", "d", "bpy", "g", "c", "m", "w", "f"]
    evaluate("D: first draft (ZQX JKV BPY)", Dk, first_key(Dk))
    Db = ["a", "e", "i", "o", "u", "s", "t", "zqp", "n", "h", "xjk", "r", "l", "d", "vby", "g", "c", "m", "w", "f"]
    evaluate("D: best split (ZQP XJK VBY)", Db, first_key(Db))
    T9 = ["abc", "def", "ghi", "jkl", "mno", "pqrs", "tuv", "wxyz"]
    evaluate("T9 (for comparison)", T9, first_key(T9))
