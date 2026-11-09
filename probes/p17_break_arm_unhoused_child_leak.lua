-- P17 the break arm and the unhoused row (extends p15's scalar fill to
-- the break class): a loop allocates a row each trip, stores it into t on
-- the fallthrough arm, and breaks BEFORE the store at the break point.
-- Ownership is minted per syntax site in program order — never simulated
-- — so the break's own mint (plan_break_exits at the break's NodeId,
-- visited by the walk before the store site below it) sees the lattice
-- state where the trip's row is allocated but not yet stored: the break
-- arm carries alm_tbl_free(child) for exactly that row.
-- Run: lua run.lua probe probes/p17_break_arm_unhoused_child_leak.lua
-- Quadrants:
--   target/release/alm                        -o /tmp/b_q1.so ... && /tmp/b_q1.so   -> #t 1, sys 2
--   target/release/alm --index-base=1         -o /tmp/b_q2.so ... && /tmp/b_q2.so   -> #t 0, sys 1
--   target/release/alm --for-limit=inclusive  -o /tmp/b_q3.so ... && /tmp/b_q3.so   -> #t 1, sys 2
--   target/release/alm --index-base=1 --for-limit=inclusive -o /tmp/b_q4.so ... && /tmp/b_q4.so
-- Verdicts (all four quadrants exit 0, zero leaks, zero double frees):
--   has_break declines detect_fill_loop (infer.rs declines the plan the
--   moment a break sits in the body), so the whole conversion battery is
--   off for this loop in every quadrant: no pre-header reserve, no fast
--   lane, no per-row ctor reserve — every store rides the generic
--   alm_tbl_set_row face and #t comes from the stores alone.
--   Q1/Q3: trip 0 stores t[0] (border 1), trip 1 breaks before the
--   store, trip 1's row is freed AT the break arm; sys == 2 at print
--   (parent + 2 rows - 1 break free), then the root free recovers
--   trip 0's housed row through the recursive walk: 0 at exit.
--   Q2/Q4: trip 0's store arrives at physical key -1 and is refused —
--   set_core reclaims that trip's row (the landed refuse-reclaim); the
--   refuses never touch the border; trip 1 breaks and frees ITS row at
--   the arm. The two reclamations claim distinct rows, so the ledger at
--   print is 1 (parent + 2 rows - 2 frees) and the refuse path never
--   double-frees the break's row nor vice versa: 0 at exit, rc 0.
-- IR pins (p17_*.ll): the break block b4 carries `call void
--   @alm_tbl_free(ptr %row)` — the per-site break-path free; after the
--   generic row store the moved binding nulls (%row_handle nulled at the
--   move site) so the next trip's decl rebuilds the row without the old
--   one escaping; the loop exit block merges the break's guard value
--   through an exit phi (the break arm consumes the event base) and
--   frees only t — the housed row is recovered by free_tbl's recursion,
--   not by a second direct free.

local t = {}
local bp = 1
for i = 0, 3 do
  local child = { i }
  if i == bp then break end
  t[i] = child
end
print(#t)
print(sys_alloc_count())
