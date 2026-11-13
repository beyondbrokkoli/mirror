-- EXPECT: 2	1
-- The latch swap over table aliases (P2, archived LOGS §3): the move
-- law admits the swap because each alias moves exactly once and the
-- rebinds land after the latch. Re-proven 2026-11-13 at c515053.
local x = {1}
local y = {2}
do
  local t0 = y
  local t1 = x
  x = t0
  y = t1
end
print(x[0], y[0])
