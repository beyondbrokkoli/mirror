-- P9 canonical: the 0-based ramp fill. i runs 0..7 (exclusive limit 8),
-- stores t[i] = i + 1, so cells 0..7 hold 1..8.
-- Run: lua run.lua probe probes/p9_canonical.lua
-- Verdicts: alm_tbl_reserve(t, 8) in the pre-header; the body store is
-- the inline fast lane (load ptr, getelementptr inbounds, store
-- !noalias; no alm_tbl_set call inside the loop); prints 8, then 1, 8.

local t = {}
for i = 0, 8 do
  t[i] = i + 1
end
print(#t)
print(t[0], t[7])
