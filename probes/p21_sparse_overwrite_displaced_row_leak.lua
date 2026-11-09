-- P21 the displaced-row overwrite leak (set_core's replace-drop arm vs
-- the parked overwrite-reclaim law). A contains_tables parent written
-- twice at the same sparse keys: the second pass's in-domain store rides
-- the generic face, and set_core's dense replace path never reads the
-- old slot word — span_grow + store_span clobber it outright — so pass
-- 1's rows + grandchildren are orphaned with zero owners. free_tbl's
-- scope-exit walk recovers only the in-slot (pass-2) tables; the
-- displaced pass-1 tables leak past the process.
-- Run: lua run.lua probe probes/p21_sparse_overwrite_displaced_row_leak.lua
-- (the exclusive base-0 faces see only 2, 4, 6 — the leak is
-- overwrite-driven, so every quadrant leaks, none crashes:)
--   exclusive base-0: exit 0, prints 7 100 0 0 13; atexit leak 6
--   base-1 exclusive: exit 0, prints 6 0 100 0 13; atexit leak 6
--   base-0 inclusive: exit 0, prints 9 100 0 0 17; atexit leak 8
--   base-1 inclusive: exit 0, prints 8 0 100 400 17; atexit leak 8
--     (studied quadrant; Q3's t[8][1][1] == 0 is the base-0 spelling —
--     the grandchild scalar at [0][0] — not a miss)
-- exe twin (base-1 inclusive), exit 0:
--   ALM_TRACE: slot 90 — leak at exit (module alm_out, sys_alloc_count() == 8)
-- .ll: zero alm_tbl_reserve call sites — the stride key declines the
-- whole conversion battery (p19's IR pin), so every store rides the
-- generic face and the leak is unminted: the fix belongs in rt.rs's
-- set_core (and any fast-lane replace face): free the valid displaced
-- child before the clobbering store when the parent carries
-- contains_tables. RT sidecar: 50:RT_ALLOC 60:RT_FREE 70:RT_FREE_CHILD
-- 90:FAIL_LEAK_DETECTED — NO RT_RESERVE_NULL (generic face only;
-- nothing contested a reserve plan, the pass-1 tables simply have no
-- second-visit reclaim).
local t = {}
for i = 1, 4 do
  t[i * 2] = { { i } }
end
for i = 1, 4 do
  t[i * 2] = { { i * 100 } }
end
print(#t)
local r = t[2]
print(r[0][0])
print(r[1][1])
print(t[8][1][1])
print(sys_alloc_count())
