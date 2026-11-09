-- P23 cross-iteration key aliasing in the claim law (LaneSink::claimed
-- keys on (loop_id, base, KeyShape)). Two DISTINCT shapes over the same
-- guard — `Guard` and `GuardOffset(k)` — mint two independent claims,
-- but their physical windows overlap across trips: at trip i, `t[i]`
-- writes slot i and `t[i + 1]` writes slot i + 1; at trip i + 1, `t[i]`
-- writes slot i + 1 — the exact slot the previous trip's offset store
-- filled. Each claim's proof ("the slot is fresh and displaces
-- nothing") only holds for the FIRST writer of a slot; the second
-- writer rides the fast lane's inline store with no runtime face in
-- between, so the displaced row + grandchild orphan — the fast-lane
-- displacement of p21's leak, reached without ever repeating a shape.
--
-- The backward twin (`t[i - 1]`) does NOT reach the lane: admission is
-- `first + k_phys >= 0`, and its first physical write is negative at
-- every base (base-0: 0 + (-1); base-1: 1 + (-2)), so the offset store
-- declines wholesale. The forward twin (+1) admits at every base.
--
-- Control a: the SAME shape twice in one loop — the first sighting
--   claims the lane (the IR: inline getelementptr+store), the second
--   hits sink.claimed, declines, rides the checked face where
--   set_core's replace-reclaim frees the displaced child (the P21
--   law). No leak.
-- Probe t: `t[i]` (Guard, k_phys 0) + `t[i + 1]` (GuardOffset(1),
--   k_phys 1), base 0 — BOTH ride inline fast stores in the emitted IR
--   (b6 stores slot %v39 = i, b8 stores slot %v49 = i + 1). The loop
--   trips 0..2 (the default for-limit is exclusive), so slots 1 and 2
--   are each written twice: trip i's offset row is clobbered by trip
--   i + 1's guard row with no free. Final slot VALUES are identical
--   either way (stdout can't tell) — the verdict is the ledger:
-- Receipts (base-0 exclusive default, 3 trips):
--   sound runtime : 20 | 2 | 0   (no atexit line)
--   gap live      : 20 | 2 | 4   (2 displaced rows + 2 grandchildren)
--                   + atexit "ALM_TRACE: slot 90 — leak at exit"
--   (the reserve stays exact either way: K_eff widens to +1, reserve
--   bound + 1 = 4 covers slots 0..3 — the clobber is an ownership
--   loss, not an out-of-span write)
-- Fix shape if the gap is live: claim on the physical window, not the
-- shape — admit only when no prior claim's window intersects
-- [first + k, bound + k] for this (loop, base), e.g. widen a
-- per-loop claimed-interval set alongside the shape set; the
-- intersecting second writer declines to the checked face, where
-- set_core's replace-reclaim already speaks the law.
-- Run: lua run.lua probe probes/p23_guard_offset_cross_iteration_alias.lua
do
  local a = {}
  for i = 0, 3 do
    a[i] = { { i } }
    a[i] = { { i * 10 } }
  end
  print(a[2][0][0])
end
do
  local t = {}
  for i = 0, 3 do
    t[i] = { { i } }
    t[i + 1] = { { i * 10 } }
  end
  print(t[2][0][0])
end
print(sys_alloc_count())
