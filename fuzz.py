#!/usr/bin/env python3
"""The differential judge harness: one source file, two engines.

Generates tuple assignments over int-valued names and int cells (with
optional tag() calls in key/value positions to pin the full evaluation
order), then runs the SAME file under the judge (luajit) and the
dialect (`alm --index-base=1`) and compares stdout byte for byte. A
reset between statements keeps every key in bounds for both engines,
so any divergence is a real semantics gap, never a bounds-law
difference. The reset lines are themselves tuple assignments — extra
coverage for free.

Born in the T2 session (2026-11-13): its first batch died on the
scalar-node ghost (a name's scalar node parked at the wrong depth, so
a later typed rebind false-rejected) — the name-target sugar
manufactured that shape from idiomatic source, which is how the bug
was found. After the fix it ran 4800 programs across 160 seeds with
ALL MATCH, and proved the 272 existing locks drift-free.

The generator is deterministic per seed (`random.Random(seed)`), so a
re-run replays the same programs; pass --seed-base to open new ground.
The dangerous pairs aim the generator at the compositions that
distinguish evaluation orders: a cell target reading a name that a
name target on the same line rebinds (and the mirror).

Usage:
    python3 fuzz.py                         # 20 batches x 60 statements
    python3 fuzz.py --batches 40 --stmts 60 # the T2 receipt shape
    python3 fuzz.py --seed-base 1000        # new programs, same shapes
    ALM_BIN=path/to/alm python3 fuzz.py     # point at another build

The dialect binary resolves in order: --bin, $ALM_BIN, then
target/release/alm and target/debug/alm next to this script (so the
repo copy finds its own build; a copy parked elsewhere passes --bin).
Divergence sources are kept and their paths printed. Exit 0 only on
ALL MATCH.
"""
import argparse
import os
import pathlib
import random
import subprocess
import sys

NAMES = ["x", "y", "z"]
# (kind, payload): name target, or cell (obj, key-expr)
CELL_KEYS = ["1", "2", "x", "y", "z", "x+1", "y+1"]
OBJS = ["t", "u"]
VALUES = ["x", "y", "z", "t[1]", "t[x]", "u[y]", "x+1", "y+z", "7", "9", "t[2]+1", "u[1]+1"]
# directed dangerous pairs: a cell target reading a name that a name
# target on the same line rebinds (and the mirror)
DANGEROUS = [
    (["t[x]", "x"], None), (["x", "t[x]"], None),
    (["u[y]", "y"], None), (["y", "u[y]"], None),
    (["t[y]", "y", "t[2]"], None), (["t[2]", "t[z]", "z"], None),
    (["z", "t[z]", "u[1]"], None),
]


def target(rng):
    if rng.random() < 0.15:
        pair, _ = rng.choice(DANGEROUS)
        return list(pair)
    n = rng.choice([2, 2, 2, 3, 3, 4])
    ts = []
    for _ in range(n):
        if rng.random() < 0.45:
            ts.append(rng.choice(NAMES))
        else:
            ts.append(f"{rng.choice(OBJS)}[{rng.choice(CELL_KEYS)}]")
    return ts


def maybe_tag(rng, e, prob):
    if rng.random() < prob:
        return f"tag(cnt, log, {e})"
    return e


def value(rng):
    v = rng.choice(VALUES)
    return maybe_tag(rng, v, 0.25)


def key_target(rng, cell):
    obj, key = cell.split("[")
    key = key.rstrip("]")
    return f"{obj}[{maybe_tag(rng, key, 0.35)}]"


def stmt(rng):
    if rng.random() < 0.30:
        ts, _ = rng.choice(DANGEROUS)
        ts = list(ts)
    else:
        ts = target(rng)
    vs = [value(rng) for _ in ts]
    rendered = []
    for t in ts:
        if "[" in t and rng.random() < 0.5:
            rendered.append(key_target(rng, t))
        else:
            rendered.append(t)
    return ", ".join(rendered) + " = " + ", ".join(vs)


PREAMBLE = """function tag(cnt, log, v)
  cnt[1] = cnt[1] + 1
  log[cnt[1]] = v
  return v
end
local x, y, z = 1, 2, 3
local t = {10, 20, 30, 40}
local u = {50, 60, 70, 80}
local cnt = {0}
local log = {0, 0, 0, 0, 0, 0, 0, 0}
"""
PRINT = "print(cnt[1], log[1], log[2], log[3], log[4], log[5], log[6], log[7], log[8], x, y, z, t[1], t[2], t[3], t[4], u[1], u[2], u[3], u[4])"
RESET = """x, y, z = 1, 2, 3
t[1], t[2], t[3], t[4] = 10, 20, 30, 40
u[1], u[2], u[3], u[4] = 50, 60, 70, 80
cnt[1] = 0
log[1], log[2], log[3], log[4], log[5], log[6], log[7], log[8] = 0, 0, 0, 0, 0, 0, 0, 0"""


def batch(seed, n_stmts):
    rng = random.Random(seed)
    parts = [PREAMBLE]
    for _ in range(n_stmts):
        parts.append(stmt(rng))
        parts.append(PRINT)
        parts.append(RESET)
    return "\n".join(parts) + "\n"


def resolve_bin(args):
    if args.bin:
        return pathlib.Path(args.bin)
    if os.environ.get("ALM_BIN"):
        return pathlib.Path(os.environ["ALM_BIN"])
    here = pathlib.Path(__file__).resolve().parent
    for cand in (here / "target/release/alm", here / "target/debug/alm"):
        if cand.exists():
            return cand
    return None


def main():
    ap = argparse.ArgumentParser(
        description="Differential fuzzer: same source under the judge "
                    "and the dialect, byte-for-byte.")
    ap.add_argument("--batches", type=int, default=20,
                    help="number of batches (each its own seed)")
    ap.add_argument("--stmts", type=int, default=60,
                    help="statements per batch")
    ap.add_argument("--seed-base", type=int, default=0,
                    help="first seed; batches run seed-base .. +batches")
    ap.add_argument("--bin", default=None,
                    help="dialect binary (default: $ALM_BIN, then the "
                         "repo build next to this script)")
    ap.add_argument("--judge", default="luajit",
                    help="judge binary (default: luajit; the T2 "
                         "receipts also held on lua5.4)")
    ap.add_argument("--out", default=None,
                    help="where to write the generated sources "
                         "(default: <script dir>/target/fuzz)")
    args = ap.parse_args()

    bin_path = resolve_bin(args)
    if bin_path is None:
        here = pathlib.Path(__file__).resolve().parent
        sys.exit(f"no dialect binary: pass --bin or ALM_BIN, or build "
                 f"one ({here}/target/release/alm)")
    out = (pathlib.Path(args.out) if args.out
           else pathlib.Path(__file__).resolve().parent / "target/fuzz")
    out.mkdir(parents=True, exist_ok=True)

    diverged = 0
    for seed in range(args.seed_base, args.seed_base + args.batches):
        src = out / f"b{seed}.lua"
        src.write_text(batch(seed, args.stmts))
        try:
            judge = subprocess.run([args.judge, str(src)],
                                   capture_output=True, text=True, timeout=30)
        except FileNotFoundError:
            sys.exit(f"judge binary '{args.judge}' not found — the "
                     "receipts were taken on luajit (lua5.4 agreed)")
        alm = subprocess.run([str(bin_path), "--index-base=1", str(src)],
                             capture_output=True, text=True, timeout=30)
        # the dialect prints build chatter (Success/ALM_TRACE) on
        # stdout; the program output follows it
        alm_out = "\n".join(l for l in alm.stdout.splitlines()
                            if not l.startswith("Success")
                            and not l.startswith("ALM_TRACE"))
        if judge.returncode != 0:
            print(f"[seed {seed}] JUDGE ERRORED — generator bug:\n{judge.stderr[:300]}")
            return 1
        if alm.returncode != 0:
            print(f"[seed {seed}] DIALECT FAILED (compile/run):\n{alm.stderr[:400]}\n  src: {src}")
            diverged += 1
            continue
        if judge.stdout.splitlines() != alm_out.splitlines():
            j, a = judge.stdout.splitlines(), alm_out.splitlines()
            for i, (jl, al) in enumerate(zip(j, a)):
                if jl != al:
                    print(f"[seed {seed}] DIVERGE at output line {i}: judge '{jl}' vs dialect '{al}'")
                    break
            else:
                print(f"[seed {seed}] DIVERGE (length): {len(j)} vs {len(a)} lines")
            diverged += 1
    total = args.batches * args.stmts
    print(f"{total} statements across {args.batches} batches: "
          + ("ALL MATCH" if diverged == 0 else f"{diverged} batches diverged"))
    return 0 if diverged == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
