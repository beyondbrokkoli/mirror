-- P22 the overflow-park → dense-grow overwrite seam (set_core's Branch
-- 2 → Branch 3 boundary). A store above `len + SPARSE_THRESHOLD` rides
-- Branch 2: the row parks in the overflow map (which IS tbl.sparse_map
-- — overflow_map lazily allocates it) while tbl.mode stays Dense. The
-- Branch-2 replace reclaims from that map correctly. But if tbl.len
-- later grows past `index - SPARSE_THRESHOLD`, the SAME index flips
-- into Branch 3's dense envelope: the span word reads 0 (the parked
-- row was never in the span), the map entry is dropped by the bare
-- `sparse_map.remove(&index)` with no reclaim, and the parked row +
-- its grandchild orphan. Branch 3 reclaims only what the span word
-- holds; it never consults the map it is about to clobber.
--
-- Reaching the seam takes a runtime-only key: a LITERAL key above the
-- threshold is seen by infer.rs check_key_threshold and flips the
-- site's LayoutVerdict to Sparse at mint (flags bit 0x01), so every
-- store rides Branch 1's map law and the seam is unreachable. A var
-- whose constant an assignment killed (`local k = 100001; k = k + 0`)
-- keeps the mint Dense and the threshold compare runtime-only.
--
-- Layout of the probe (base-0 exclusive default, no loops — every
-- store rides the generic face, no reserve plans involved):
--   control u: BOTH writes above the threshold, no dense growth in
--     between. The replace stays in Branch 2, whose map consult frees
--     the displaced row — sound by the P21 law, mid-block sys 3
--     (u + pass-2 row + grandchild), block exit drains to 0.
--   probe t: write 1 parks in the map (100001 > 0 + 100000), `t[1] =
--     { { 7 } }` (a row — the cells-hold-one-kind law forbids growing a
--     row-housing table with a scalar) spans t to len 8, so write 2
--     (100001 <= 8 + 100000) flips to Branch 3: span_grow to 100002
--     slots, load_slot reads the fresh zero word, and
--     `sparse_map.remove(&100001)` drops the parked lane untouched.
-- Receipts (default quadrant):
--   sound runtime : 1000 2000 3 | 1000 3 5 2000 5 | 0
--   gap live      : 1000 2000 3 | 1000 3 5 2000 7 | 2
--                   + atexit "ALM_TRACE: slot 90 — leak at exit"
--                     (the parked row + grandchild survive every walk:
--                      the exit walk drains t's dense children, but
--                      the map entry is already gone)
-- Fix shape if the gap is live: Branch 3 must feed the removed lane
-- through reclaim_displaced_child when owns_cell —
--   let old = if !tbl.sparse_map.is_null()
--       { (&mut *tbl.sparse_map).remove(&index) } else { None };
--   (reclaim on Some(old) alongside the span word's own reclaim — the
--   same law the span word already obeys).
-- Run: lua run.lua probe probes/p22_overflow_park_dense_grow_leak.lua
do
  local j = 100001
  j = j + 0
  local u = {}
  u[j] = { { 1000 } }
  print(u[j][0][0])
  u[j] = { { 2000 } }
  print(u[j][0][0])
  print(sys_alloc_count())
end
do
  local k = 100001
  k = k + 0
  local t = {}
  t[k] = { { 1000 } }
  print(t[k][0][0])
  print(sys_alloc_count())
  t[1] = { { 7 } }
  print(sys_alloc_count())
  t[k] = { { 2000 } }
  print(t[k][0][0])
  print(sys_alloc_count())
end
print(sys_alloc_count())
