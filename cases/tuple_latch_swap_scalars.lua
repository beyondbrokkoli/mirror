-- EXPECT: 2	1
-- The latch swap, hand-written: the exact composition customer #1
-- (name-target tuple assignment, the T2 lane) desugars 'x, y = y, x'
-- into. Probe-proven 2026-11-05 (P1, archived LOGS §3) and re-proven
-- 2026-11-13 at c515053; pinned in the corpus so the T2 desugar has a
-- green oracle that predates it.
local x = 1
local y = 2
do
  local t0 = y
  local t1 = x
  x = t0
  y = t1
end
print(x, y)
