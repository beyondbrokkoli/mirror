-- FLAGS: --index-base=1
-- EXPECT: 4
-- The recorded stale-min hypothesis, probed 2026-11-10: the entry's
-- lower bound is the min over all recorded shapes (0 from `s = 0`,
-- 1 from `s = 1`), so the folded physical entry (0 - 1) fails the
-- gate and the store rides the checked arm — though the last shape
-- provably admits. The declined store contributes no offset, so the
-- reserve keeps the raw bound and the border over-reports by the
-- base shift: #t is 4 while keys 1..3 are written. Current law.
local t = {}
local s = 0
s = 1
local i = s
while i < 4 do
  t[i] = i
  i = i + 1
end
print(#t)
