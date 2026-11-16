-- EXPECT: 1
-- XFAIL: judge_divergence_audit/reserve-border — #t reports the
-- reserve (2 cells for keys 0..1); the judge reports the border (1).
-- Flip with the # border law.
local t = {}
t[0] = 1
t[1] = 2
print(#t)
