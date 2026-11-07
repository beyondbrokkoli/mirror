-- P9 shifted: the 1-based shadow spelling of the same fill. i runs 1..8
-- (exclusive limit 9), stores t[i - 1] = i, so cells 0..7 hold 1..8 --
-- identical memory to p9_canonical.
-- Run: lua run.lua probe probes/p9_shifted.lua
-- Verdicts: the key i - 1 is KeyShape::GuardOffset(-1) and re-proves
-- through the gate (entry_const 1, 1 + (-1) = 0 >= 0), so the store
-- rides the same inline fast lane with exactly one sub in the key lane
-- (a rejected gate would show as an alm_tbl_set call instead); the
-- tight reserve covers bound + K_eff = 9 - 1 = 8 cells (the settled P9
-- decision; the clamp-era before-picture printed 9); prints 8, then
-- 1, 8. This file is also the golden IR shape for --index-base=1: the
-- base-1 spelling `for i = 1, 9 do t[i] = i end` must normalize to
-- this exact IR (see probes/p9_base1.lua).
-- Pair diff: python3 probes/ir_delta.py target/probe/p9_canonical.ll
-- target/probe/p9_shifted.ll

local t = {}
for i = 1, 9 do
  t[i - 1] = i
end
print(#t)
print(t[0], t[7])
