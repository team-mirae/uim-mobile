"""Letter-group (spelling-based) ambiguous layouts, e.g. the first phonetic sketch and T9.
Usage: python3 letter_groups.py data/count_1w.txt [N]"""
import sys, re, collections
N = int(sys.argv[2]) if len(sys.argv) > 2 else 20000
words = []
for line in open(sys.argv[1]):
    w, c = line.split("\t")
    if re.fullmatch(r"[a-z]+", w): words.append((w, int(c)))
    if len(words) >= N: break
tot = sum(c for _, c in words)

def layout_keys(spec):
    # spec: list of key specs; "ab" = set {a,b}; "`ing`" = literal string
    keys = []
    for k in spec:
        if k.startswith("`"): keys.append(("str", k.strip("`")))
        elif k == "*": keys.append(("set", "abcdefghijklmnopqrstuvwxyz"))
        else: keys.append(("set", k))
    return keys

def encode(w, keys, prefer):
    # canonical sequence the user would type: use string keys greedily, else prefer[letter]
    out = []; i = 0
    while i < len(w):
        hit = None
        for idx, (t, v) in enumerate(keys):
            if t == "str" and w.startswith(v, i): hit = (idx, len(v)); break
        if hit: out.append(hit[0]); i += hit[1]; continue
        out.append(prefer(w, i)); i += 1
    return tuple(out)

def run(name, spec, prefer_map, soft_c=None):
    keys = layout_keys(spec)
    def prefer(w, i):
        ch = w[i]
        if soft_c and ch == "c":
            nxt = w[i+1] if i+1 < len(w) else ""
            return soft_c[1] if nxt in ("e", "i", "y") else soft_c[0]
        if ch in prefer_map: return prefer_map[ch]
        cands = [idx for idx, (t, v) in enumerate(keys) if t == "set" and ch in v and v != "abcdefghijklmnopqrstuvwxyz"]
        return cands[0] if cands else next(idx for idx,(t,v) in enumerate(keys) if v=="abcdefghijklmnopqrstuvwxyz")
    seqs = {w: encode(w, keys, prefer) for w, _ in words}
    trie = {}
    for w, c in words:
        node = trie
        for ch in w: node = node.setdefault(ch, {})
        node["$"] = c
    def find(seq):
        out = []
        def go(node, j, acc):
            if j == len(seq):
                if "$" in node: out.append((acc, node["$"]))
                return
            t, v = keys[seq[j]]
            if t == "str":
                n = node
                for ch in v:
                    n = n.get(ch)
                    if n is None: return
                go(n, j+1, acc+v)
            else:
                for ch in v:
                    if ch in node: go(node[ch], j+1, acc+ch)
        go(trie, 0, "")
        out.sort(key=lambda t: -t[1]); return [x for x, _ in out]
    r1 = r4 = 0; ranks = []; cache = {}
    for w, c in words:
        s = seqs[w]
        if s not in cache: cache[s] = find(s)
        rank = cache[s].index(w) + 1
        if rank == 1: r1 += c
        if rank <= 4: r4 += c
        ranks.append((rank, w, c))
    nkeys = len(spec)
    print(f"{name:34s} keys={nkeys:2d}  top1={100*r1/tot:5.1f}%  top4={100*r4/tot:5.1f}%  (freq-weighted, top {N} words)")
    return cache, seqs, ranks

if __name__ == "__main__":
    P = ["a","A","e","i","o","u", "qck","w","rl","ts","y","`ing`", "sx","d","fph","gj","m","`er`", "x","cs","v","b","n","*"]
    def go(name, spec, s_key, x_key, hard, soft):
        ix = {k: i for i, k in enumerate(spec)}
        run(name, spec, {"s": ix[s_key], "x": ix[x_key]}, soft_c=(ix[hard], ix[soft]))
    go("phonetic (as given), s on ts", P, "ts", "x", "qck", "cs")
    go("phonetic, s on sx", P, "sx", "x", "qck", "cs")
    go("phonetic, without ing/er keys", [k for k in P if not k.startswith("`")], "ts", "x", "qck", "cs")
    run("T9 (8 letter keys)", ["abc","def","ghi","jkl","mno","pqrs","tuv","wxyz"], {})
    run("QWERTY exact (26 keys)", list("abcdefghijklmnopqrstuvwxyz"), {})
