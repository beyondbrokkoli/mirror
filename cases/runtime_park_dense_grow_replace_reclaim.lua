-- EXPECT: 1000
-- EXPECT: 2000
-- EXPECT: 3
-- EXPECT: 1000
-- EXPECT: 3
-- EXPECT: 5
-- EXPECT: 2000
-- EXPECT: 5
-- EXPECT: 0
-- P22 promoted (probes/p22_overflow_park_dense_grow_leak.lua): the
-- overflow-park -> dense-grow overwrite seam. The first store parks in
-- the overflow map (100001 rides Branch 2, tbl.mode stays Dense), then
-- a dense row spans t to len 8, and the same key flips into Branch 3's
-- envelope: the span word reads fresh zero, and the map lane the
-- replace drops must be reclaimed like the span word's own. Branch 3
-- feeds the removed lane through reclaim_displaced_child when
-- owns_cell, so the parked row + grandchild never survive the walk.
-- The literal-key store declines at mint (the threshold check), so
-- every store rides the generic face — no reserve plans involved.
-- Control u: both writes above the threshold with no dense growth in
-- between — Branch 2's map consult frees the displaced row (P21's
-- law), mid-block sys 3 (u + row + grandchild). Probe t: park, dense
-- grow to 5 (t's row pair added), replace through Branch 3 back at 5
-- (the parked pair reclaimed, the new pair occupies the envelope).
-- Pre-fix ledger: control 3, probe 3 5 7, exit 2 with an atexit leak
-- tell (slot 90). Every block drains to 0.
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
