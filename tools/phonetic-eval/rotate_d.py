"""Candidate D: try every split of the 9 grouped letters (z q x j k v b p y) into 3 keys of 3.
Scores all 280 splits by 1-gram rank, then the best 10 by 2-gram rank."""
import itertools, io, contextlib, re
exec(open(__file__.replace("rotate_d.py", "spelled_keys.py")).read().split('if __name__')[0])
SINGLE = ["a", "e", "i", "o", "u", "s", "t", "n", "h", "r", "l", "d", "g", "c", "m", "w", "f"]
RARE = "zqxjkvbpy"

def splits():
    first = RARE[0]
    for a in itertools.combinations(RARE[1:], 2):
        g1 = first + "".join(a); rest = [c for c in RARE if c not in g1]
        f2 = rest[0]
        for b in itertools.combinations(rest[1:], 2):
            g2 = f2 + "".join(b); g3 = "".join(c for c in rest if c not in g2)
            yield (g1, g2, g3)

def score1(groups):
    keys = SINGLE + list(groups)
    k_of = {ch: i for i, ks in enumerate(keys) for ch in ks}
    buckets = collections.defaultdict(list)
    for w in vocab: buckets[tuple(k_of[c] for c in w)].append(w)
    miss = 0
    for g in buckets.values():
        g.sort(key=lambda x: -uni[x]); miss += sum(uni[x] for x in g[1:])
    return 1 - miss / tot1

def score2(groups):
    keys = SINGLE + list(groups)
    f = io.StringIO()
    with contextlib.redirect_stdout(f): evaluate("", keys, first_key(keys))
    m = re.search(r"2-gram: first\s+([\d.]+)%", f.getvalue()); return float(m.group(1))

if __name__ == "__main__":
    allsp = list(splits())
    res = sorted(((score1(g), g) for g in allsp), reverse=True)
    print(f"{len(allsp)} splits. Worst: {' '.join(res[-1][1]).upper()} {100*res[-1][0]:.2f}%")
    cur = ("zqx", "jkv", "bpy")
    cs = score1(cur)
    print(f"current ZQX JKV BPY: 1-gram first {100*cs:.2f}% (rank {1+sum(1 for s,_ in res if s > cs)} of {len(res)})")
    print("best 10 (1-gram first, then 2-gram first):")
    for s, g in res[:10]:
        print(f"  {' '.join(x.upper() for x in g):14s} 1-gram {100*s:.2f}%   2-gram {score2(g):.1f}%")
    print(f"  {'ZQX JKV BPY':14s} 1-gram {100*cs:.2f}%   2-gram {score2(cur):.1f}%  (current)")
