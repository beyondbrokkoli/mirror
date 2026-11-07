-- FLAGS: --index-base=1
-- EXPECT: 9
-- EXPECT: 9
-- EXPECT: 1	8	1	8
-- key_offset is per-loop by design: widen keeps the widest admitted
-- offset, and every pre-header reserve rides the same widened
-- register -- an envelope, because covering bound + K_eff for every
-- admitted key is the safety law and tightness (border equals logical
-- length) holds only while the loop's admitted keys share one offset.
-- Here a[i] sits at -1 and b[i + 1] at 0, the envelope takes 0, and
-- both reserves cover 9: b's border is exact while a's over-reports
-- by the 1-cell difference. Every value still lands through its own
-- translated key; the same envelope governs base 0 (a[i] at 0 against
-- b[i + 2] at 2 widens both to bound + 2).
local a = {}
local b = {}
for i = 1, 9 do
  a[i] = i
  b[i + 1] = i
end
print(#a)
print(#b)
print(a[1], a[8], b[2], b[9])
