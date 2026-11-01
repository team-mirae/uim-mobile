#!/usr/bin/env python3
"""Builds engine/data/en-acronyms.txt: the words EN predictive (SPEC §3.3) ranks first when
everything typed is upper case (API for A·ZQP·I).

    python3 -I tools/en-acronyms/build_acronyms.py

Source: the Hunspell en_US dictionary (LibreOffice/dictionaries, pinned below), derived from SCOWL.
Its all-caps stems of 2+ letters are the acronyms, unless also a lower-case entry. Counts come from Norvig's count_1w.txt (the
list engine/data/en-words.txt is cut from), 0 when absent. SCOWL's notice is written to
engine/data/LICENSE-scowl.txt, and must ship with the data.

Output: "word\\tcount" lines, the word in lower case, sorted by count, highest first.
"""
import os
import re
import subprocess
import sys

COMMIT = "4f1df64b2e8e14751aa7c2205d170ca016071315"  # LibreOffice/dictionaries master, 2026-11-02
BASE = f"https://raw.githubusercontent.com/LibreOffice/dictionaries/{COMMIT}/en/"
COUNTS_URL = "https://norvig.com/ngrams/count_1w.txt"

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
DATA = os.path.join(ROOT, "engine", "data")
CACHE = os.path.join(HERE, "cache")


def fetch(name, url):
    path = os.path.join(CACHE, name)
    if not os.path.exists(path):
        os.makedirs(CACHE, exist_ok=True)
        print("fetching", name, file=sys.stderr)
        subprocess.run(["curl", "-sSfL", "-o", path + ".part", url], check=True)
        os.replace(path + ".part", path)
    return path


def main():
    dic = fetch("en_US.dic", BASE + "en_US.dic")
    readme = fetch("README_en_US.txt", BASE + "README_en_US.txt")
    local = os.path.join(ROOT, "tools", "phonetic-eval", "data", "count_1w.txt")
    counts_path = local if os.path.exists(local) else fetch("count_1w.txt", COUNTS_URL)
    counts = {}
    with open(counts_path, encoding="utf-8") as f:
        for line in f:
            w, c = line.split("\t")
            counts[w] = int(c)
    stems = set()
    with open(dic, encoding="utf-8") as f:
        next(f)  # the entry count
        for line in f:
            stems.add(line.split("/")[0].strip())
    # Not when the word is also a lower-case entry (IN Indiana, WHO, ME Maine): its count is the
    # ordinary word's, and all caps there is more likely emphasis than the acronym.
    words = {s.lower() for s in stems if re.fullmatch(r"[A-Z]{2,}", s) and s.lower() not in stems}
    rows = sorted(((counts.get(w, 0), w) for w in words), key=lambda r: (-r[0], r[1]))
    with open(os.path.join(DATA, "en-acronyms.txt"), "w", encoding="utf-8") as f:
        for c, w in rows:
            f.write(f"{w}\t{c}\n")
    with open(readme, encoding="utf-8") as f:
        text = f.read()
    notice = text[text.index("COPYRIGHT, SOURCES, and CREDITS:"):]
    with open(os.path.join(DATA, "LICENSE-scowl.txt"), "w", encoding="utf-8") as f:
        f.write(f"engine/data/en-acronyms.txt is built by tools/en-acronyms/build_acronyms.py from the Hunspell\n"
                f"en_US dictionary (https://github.com/LibreOffice/dictionaries, en/, commit {COMMIT}),\n"
                f"which is derived from SCOWL. Its notice (README_en_US.txt, verbatim from this point):\n\n{notice}")
    print(f"en-acronyms.txt: {len(rows)} words, {sum(1 for c, _ in rows if c)} with counts", file=sys.stderr)


if __name__ == "__main__":
    main()
