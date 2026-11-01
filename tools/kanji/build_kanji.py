#!/usr/bin/env python3
"""Builds engine/data/kanji.bin, JA's kana-kanji conversion data (SPEC §6, §8 M6).

Ported from jis-moonshot (engine/tools/kanji/build_kanji.py); same format, so engine/src/kanji.rs
reads either.

The source is Mozc's open-source dictionary (google/mozc, src/data/dictionary_oss) at a pinned
commit: IPAdic (NAIST, a permissive licence that requires its notice), the Okinawa dictionary
(public domain) and Google's additions (BSD-3-Clause, as Mozc). Their notices are written to
engine/data/LICENSE-mozc.txt next to the data.

    python3 -I tools/kanji/build_kanji.py    # fetch (cached in tools/kanji/mozc), prune, write

It also copies Mozc's evaluation set to engine/tests/data/kanji-evaluation.tsv, which
engine/tests/kanji.rs converts (the accuracy floor there is this cut's).

Pruning (measured against Mozc's evaluation.tsv, exact sentence matches: the whole dictionary
79%, this cut 70% at about a third of the entries): every entry of cost <= KEEP_COST, plus the
TOP_PER_READING cheapest surfaces of each reading up to RARE_COST. Readings longer than
MAX_READING kana are dropped (the converter looks words up at most that long).

Format (little-endian), version 1:
    "JKKJ" u32 version | u32 ids | u32 unknown_id | u32 runs | u32 readings | u32 entries |
    u32 surface_bytes
    matrix: `runs` x (u8 value, u8 length 1-255), row-major, run-length encoded; cost = value
            * 64. Row = the left word's right id, column = the right word's left id.
    readings, sorted by UTF-8 bytes, front-coded: u8 bytes shared with the reading before, u8
            byte length of the rest, the rest, u8 entry count
    entries, in reading order: u16 left id, u16 right id, u16 cost, u32 surface
            (0xFFFFFFFF: the reading itself, 0xFFFFFFFE: the reading in katakana, else an offset
            into the surfaces)
    surfaces: u8 byte length, the bytes
"""

import argparse
import collections
import os
import struct
import subprocess
import sys

COMMIT = "e5f1494923a0a624d01f19c50d57bbdfa03ea5ba"  # google/mozc master, 2026-11-01
BASE = f"https://raw.githubusercontent.com/google/mozc/{COMMIT}/src/data/dictionary_oss/"
LICENSE_URL = f"https://raw.githubusercontent.com/google/mozc/{COMMIT}/LICENSE"
FILES = ["README.txt", "id.def", "connection_single_column.txt", "evaluation.tsv"] + [f"dictionary0{i}.txt" for i in range(10)]

KEEP_COST = 5500
TOP_PER_READING = 2
RARE_COST = 7000
MAX_READING = 8
STEP = 64  # connection cost quantum: one byte per cell, no measurable loss
UNKNOWN_POS = "名詞,一般,*,*,*,*,*"

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.join(os.path.dirname(os.path.dirname(HERE)), "engine")
CACHE = os.path.join(HERE, "mozc")


def fetch():
    """Downloads what isn't cached yet, with curl (Python's own certificates aren't always set up)."""
    os.makedirs(CACHE, exist_ok=True)
    for name, url in [(n, BASE + n) for n in FILES] + [("LICENSE", LICENSE_URL)]:
        path = os.path.join(CACHE, name)
        if not os.path.exists(path):
            print("fetching", name, file=sys.stderr)
            subprocess.run(["curl", "-sSfL", "-o", path + ".part", url], check=True)
            os.replace(path + ".part", path)


def katakana(s):
    return "".join(chr(ord(c) + 0x60) if "ぁ" <= c <= "ゖ" else c for c in s)


def load():
    with open(os.path.join(CACHE, "connection_single_column.txt")) as f:
        n = int(f.readline())
        matrix = [int(line) for line in f]
    assert len(matrix) == n * n, "connection matrix size"
    unknown = None
    with open(os.path.join(CACHE, "id.def"), encoding="utf-8") as f:
        for line in f:
            i, pos = line.rstrip("\n").split(" ", 1)
            if pos == UNKNOWN_POS:
                unknown = int(i)
    assert unknown is not None, "no id for " + UNKNOWN_POS
    rows = []
    for i in range(10):
        with open(os.path.join(CACHE, f"dictionary0{i}.txt"), encoding="utf-8") as f:
            for line in f:
                reading, left, right, cost, surface = line.rstrip("\n").split("\t")[:5]
                rows.append((reading, int(left), int(right), int(cost), surface))
    return n, matrix, unknown, rows


def prune(rows):
    by = collections.defaultdict(list)
    for reading, left, right, cost, surface in rows:
        if len(reading) <= MAX_READING and len(surface.encode()) <= 255 and len(reading.encode()) <= 255:
            by[reading].append((cost, left, right, surface))
    kept = {}
    for reading, entries in by.items():
        entries.sort()
        seen, out, ids = set(), [], set()
        for cost, left, right, surface in entries:
            if (left, right, surface) in ids:
                continue
            if cost <= KEEP_COST or (len(seen) < TOP_PER_READING and cost <= RARE_COST):
                out.append((left, right, cost, surface))
                ids.add((left, right, surface))
                seen.add(surface)
        if out:
            kept[reading] = out[:255]
    return kept


def encode(n, matrix, unknown, kept):
    runs = []
    for row in range(n):
        cells = [min(v, 255 * STEP) // STEP for v in matrix[row * n:(row + 1) * n]]
        value, length = cells[0], 0
        for v in cells:
            if v == value and length < 0xFF:
                length += 1
            else:
                runs.append((value, length))
                value, length = v, 1
        runs.append((value, length))
    surfaces, offsets = bytearray(), {}
    readings, entries = bytearray(), bytearray()
    count, previous = 0, b""
    for reading in sorted(kept, key=lambda r: r.encode()):
        raw = reading.encode()
        shared = 0
        while shared < min(len(raw), len(previous)) and raw[shared] == previous[shared]:
            shared += 1
        readings += bytes([shared, len(raw) - shared]) + raw[shared:] + bytes([len(kept[reading])])
        previous = raw
        for left, right, cost, surface in kept[reading]:
            if surface == reading:
                ref = 0xFFFFFFFF
            elif surface == katakana(reading):
                ref = 0xFFFFFFFE
            else:
                if surface not in offsets:
                    offsets[surface] = len(surfaces)
                    s = surface.encode()
                    surfaces += bytes([len(s)]) + s
                ref = offsets[surface]
            entries += struct.pack("<HHHI", left, right, cost, ref)
            count += 1
    out = bytearray(b"JKKJ")
    out += struct.pack("<7I", 1, n, unknown, len(runs), len(kept), count, len(surfaces))
    for value, length in runs:
        out += struct.pack("<BB", value, length)
    return bytes(out + readings + entries + surfaces), count


def write_license():
    with open(os.path.join(CACHE, "README.txt"), encoding="utf-8") as f:
        notices = f.read()
    with open(os.path.join(CACHE, "LICENSE"), encoding="utf-8") as f:
        mozc = f.read()
    with open(os.path.join(ENGINE, "data", "LICENSE-mozc.txt"), "w", encoding="utf-8") as f:
        f.write(f"""engine/data/kanji.bin is built by tools/kanji/build_kanji.py from Mozc's
open-source dictionary (https://github.com/google/mozc, src/data/dictionary_oss, commit
{COMMIT}), pruned and re-encoded.

Mozc's licence (its LICENSE file, verbatim):

{mozc}
The dictionary's own sources and their licences (Mozc's dictionary_oss/README.txt, verbatim):

-------------------------------------------------------------------------------
{notices}""")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=os.path.join(ENGINE, "data", "kanji.bin"))
    args = ap.parse_args()
    fetch()
    n, matrix, unknown, rows = load()
    kept = prune(rows)
    data, count = encode(n, matrix, unknown, kept)
    with open(args.out, "wb") as f:
        f.write(data)
    write_license()
    evaluation = os.path.join(ENGINE, "tests", "data", "kanji-evaluation.tsv")
    with open(os.path.join(CACHE, "evaluation.tsv"), encoding="utf-8") as src, open(evaluation, "w", encoding="utf-8") as dst:
        dst.write(f"# Mozc's dictionary_oss/evaluation.tsv (commit {COMMIT}, BSD-3-Clause, see engine/data/LICENSE-mozc.txt)\n")
        dst.write(src.read())
    print(f"{args.out}: {len(data)} bytes, {len(kept)} readings, {count} entries", file=sys.stderr)


if __name__ == "__main__":
    main()
