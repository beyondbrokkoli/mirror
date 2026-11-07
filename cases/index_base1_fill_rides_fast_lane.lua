-- FLAGS: --index-base=1
-- EXPECT: 8
-- EXPECT: 1	8
-- The base-1 golden fill: the bare guard key t[i] is Guard with
-- physical offset -1, the gate re-proves (entry 1, 1 - 1 = 0 >= 0),
-- and the store rides the inline fast lane with exactly one sub in the
-- key lane (the seam's translation). The tight reserve covers
-- bound + K_eff = 9 - 1 = 8 cells, so #t equals the logical length.
-- This case's locked IR is the battery-side twin of probes/p9_base1.
local t = {}
for i = 1, 9 do
  t[i] = i
end
print(#t)
print(t[1], t[8])
