-- EXPECT: 0
-- XFAIL: judge_divergence_audit/reserve-border — #t reports the
-- reserve (6 cells for keys 0..5); the judge reports the border (0,
-- t[1] is nil). Flip with the # border law.
local t = {[0] = 1, [5] = 2}
print(#t)
