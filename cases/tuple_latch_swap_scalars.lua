-- EXPECT: 2	1
-- The latch swap, hand-written: the composition T2's 'x, y = y, x'
-- desugars into — same temps and values; the sugar emits the stores
-- right-to-left (the judge's order; distinct names never observe it).
-- Probe-proven 2026-11-05 (P1, archived LOGS §3), re-proven
-- 2026-11-13 at c515053, and oracle to tuple_name_swap since T2.
local x = 1
local y = 2
do
  local t0 = y
  local t1 = x
  x = t0
  y = t1
end
print(x, y)
