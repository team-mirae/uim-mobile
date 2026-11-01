"""Sound-key layout for EN predictive (SPEC §3.3): how often the intended word ranks first / in the
4-candidate bar, by 1-gram and 2-gram frequency. Needs data/ (see README.md)."""
import re, sys, collections
import os
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data") + "/"
N = 20000
# ---- data
uni = {}
for line in open(D + "count_1w.txt"):
    w, c = line.split("\t")
    if re.fullmatch(r"[a-z]+", w): uni[w] = int(c)
pron = collections.defaultdict(list)
for line in open(D + "cmudict.dict"):
    line = line.split("#")[0].strip()
    if not line: continue
    w, *ph = line.split()
    w = re.sub(r"\(\d+\)$", "", w)
    if re.fullmatch(r"[a-z]+", w): pron[w].append(["AH0" if p == "AH0" else ("AHs" if p.startswith("AH") else re.sub(r"\d", "", p)) for p in ph])
vocab = []
for w, c in sorted(uni.items(), key=lambda t: -t[1]):
    if w in pron: vocab.append(w)
    if len(vocab) >= N: break
V = set(vocab); tot1 = sum(uni[w] for w in vocab)

CONS = {"P": "PB", "B": "PB", "T": "TD", "D": "TD", "K": "KG", "G": "KG",
        "F": "FVH", "V": "FVH", "HH": "FVH", "S": "SZ", "Z": "SZ",
        "SH": "SH", "ZH": "SH", "CH": "SH", "JH": "SH", "TH": "TH", "DH": "TH",
        "M": "MN", "N": "MN", "NG": "MN", "R": "RL", "L": "RL", "ER": "RL", "W": "WY", "Y": "WY"}
VOWELS = {"AA", "EY", "EH", "IH", "OW", "UW", "AE", "AW", "IY", "AY", "AO", "UH", "AH", "OY"}

FLICK_V = {"AA": "あ", "AHs": "ʌ", "AH0": "~", "AO": "o", "UW": "う"}   # consonant flick: E W W N S

def make_coder(extra, schwa_any, strong_ah="あ", flick=False):
    keys = {"AA": ["あ"], "EY": ["에이"], "EH": ["에"], "IH": ["い"], "OW": ["오"], "UW": ["う"],
            "IY": ["イー"], "AO": ["o"], "UH": ["ü"], "OY": ["o", "い"]}
    fallback = {"AE": ["あ"], "AW": ["あ", "う"], "AY": ["あ", "い"]}
    for s_ in ["AE", "AW", "AY"]:
        keys[s_] = [s_] if s_ in extra else fallback[s_]
    keys["AHs"] = ["AH"] if "AH" in extra else [strong_ah]
    keys["AH0"] = ["~"] if schwa_any else ["あ"]
    def code(ph):
        out = []
        for i, p in enumerate(ph):
            if p in CONS: out.append(CONS[p])
            elif flick and p == "AHs" and i > 0 and ph[i-1] in CONS: out.append("ʌ")
            else: out += keys[p]
        return tuple(out)
    def gestures(ph):
        # taps/flicks to type ph: with flick, a consonant + (あ ʌ/schwa o(au) う) is one gesture
        n = len(code(ph)); 
        if not flick: return n
        k = 0
        for i, p in enumerate(ph):
            if p in FLICK_V and i > 0 and ph[i-1] in CONS: k += 1
        return n - k
    code.gestures = gestures
    return code

VKEYS = {"あ", "에이", "에", "い", "오", "う", "イー", "o", "ü", "AE", "AW", "AY", "AH"}

def evaluate(name, extra, schwa_any, strong_ah="あ", flick=False):
    code = make_coder(extra, schwa_any, strong_ah, flick)
    trie = {}
    for w in vocab:
        for ph in pron[w]:
            n = trie
            for k in code(ph): n = n.setdefault(k, {})
            n.setdefault("$", set()).add(w)
    def find(seq):
        out = set()
        def go(n, j):
            if j == len(seq):
                out.update(n.get("$", ())); return
            k = seq[j]
            if k == "~":   # typed schwa: any vowel key or schwa slot
                for kk, nn in n.items():
                    if kk != "$" and (kk in VKEYS or kk in ("~", "ʌ")): go(nn, j + 1)
            else:
                if k in n: go(n[k], j + 1)
                if (k in VKEYS or k == "ʌ") and "~" in n: go(n["~"], j + 1)
        go(trie, 0); return out
    cache = {}
    def cands(w):
        s = code(pron[w][0])
        if s not in cache: cache[s] = find(s)
        return cache[s]
    r1 = r4 = 0; bad = collections.Counter()
    for w in vocab:
        c = sorted(cands(w), key=lambda x: -uni[x]); r = c.index(w) + 1
        if r == 1: r1 += uni[w]
        if r <= 4: r4 += uni[w]
        else: bad[(w, tuple(c[:3]))] += uni[w]
    # bigram
    b1 = b4 = btot = 0
    for line in open(D + "count_2w.txt"):
        ab, c = line.rsplit("\t", 1); c = int(c)
        p = ab.split(" ")
        if len(p) != 2: continue
        a, b = p
        if a not in uni or b not in V: continue
        cs = cands(b)
        def sc(x):
            k = big.get((a, x))
            return k / uni[a] if k else 0.4 * uni[x] / tot1
        btot += c
        ranked = sorted(cs, key=lambda x: -sc(x)); r = ranked.index(b) + 1
        if r == 1: b1 += c
        if r <= 4: b4 += c
    g = sum(uni[w] * code.gestures(pron[w][0]) for w in vocab) / tot1
    L = sum(uni[w] * len(w) for w in vocab) / tot1
    print(f"{name:44s} taps/word {g:4.2f} (letters {L:4.2f}) | 1-gram: first {100*r1/tot1:5.1f}%  bar {100*r4/tot1:5.1f}% | 2-gram: first {100*b1/btot:5.1f}%  bar {100*b4/btot:5.1f}%")
    return bad

big = {}
for line in open(D + "count_2w.txt"):
    ab, c = line.rsplit("\t", 1); p = ab.split(" ")
    if len(p) == 2: big[(p[0], p[1])] = int(c)

if __name__ == "__main__":
    print(f"vocab {len(vocab)} words with pronunciations")
    which = sys.argv[1] if len(sys.argv) > 1 else "flick"
    if which == "variants":
        evaluate("A: æ aʊ aɪ; ʌ typed あ; schwa any vowel", {"AE", "AW", "AY"}, True)
        evaluate("A': æ aʊ aɪ; ʌ typed あ; schwa typed あ", {"AE", "AW", "AY"}, False)
        evaluate("B: æ aɪ ʌ; aʊ = あ+う; schwa any vowel", {"AE", "AY", "AH"}, True)
        evaluate("B': æ aɪ ʌ; aʊ = あ+う; schwa typed あ", {"AE", "AY", "AH"}, False)
    else:
        evaluate("taps only (SPEC §3.3)", {"AE", "AW", "AY"}, True)
        evaluate("consonant flick (E あ, W ʌ, N o(au), S う)", {"AE", "AW", "AY"}, True, flick=True)
