#!/usr/bin/env python3
"""Normalized IR delta for probe pairs (replaces the archived
/home/halim/mirror_docs/tools/ir_delta.py).

Strips positional numbering (registers %vN, seams %tsN, blocks %bN,
debug metadata) so two compilations of sibling sources diff by
structure only. Dbg binds and the metadata tail are dropped: their
numbering is positional and attribution has its own law cases.

Usage: python3 probes/ir_delta.py A.ll B.ll
Exit 0 on identical normalized IR, 1 on any delta (prints the diff).
"""

import difflib
import re
import sys


def normalize(text: str) -> list[str]:
    out: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("!") or stripped.startswith("attributes"):
            continue  # metadata definitions: numbering is positional
        if "llvm.dbg" in line:
            continue  # commentary binds; dropped for the structural diff
        line = re.sub(r"!dbg !\d+", "", line)
        line = re.sub(r"%ts[\d.]*", "%ts_", line)
        line = re.sub(r"%bts\d+\w*", "%bts_", line)
        line = re.sub(r"%b\d+", "%b_", line)
        line = re.sub(r"%v\d+", "%v_", line)
        line = re.sub(r"!\d+", "!_", line)
        line = re.sub(r"\bbts\d+\w*:", "bts_:", line)
        line = re.sub(r"\bb\d+:", "b_:", line)
        line = re.sub(r"\bts[\d.]+", "ts_", line)
        out.append(line.rstrip())
    # Drop runs of blank lines left by removed binds.
    return [l for l in out if l]


def main() -> int:
    if len(sys.argv) != 3:
        sys.stderr.write(__doc__)
        return 2
    with open(sys.argv[1]) as f:
        a = normalize(f.read())
    with open(sys.argv[2]) as f:
        b = normalize(f.read())
    diff = list(difflib.unified_diff(a, b, sys.argv[1], sys.argv[2], lineterm=""))
    if not diff:
        print("normalized IR identical")
        return 0
    print("\n".join(diff))
    subs = sum(1 for l in diff if " sub " in l)
    print(f"\ndelta: {subs} line(s) carrying a sub")
    return 1


if __name__ == "__main__":
    sys.exit(main())
