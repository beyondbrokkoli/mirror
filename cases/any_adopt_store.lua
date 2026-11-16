-- EXPECT: five
-- EXPECT: 2
-- EXPECT: 4
-- XFAIL: judge_divergence_audit/reserve-border — #t reports the
-- reserve after the grow (3); the judge reports the border (2). Flip
-- with the # border law.
local t = {7}
t[2] = "five"
print(t[2])
print(#t)
print(#t[2])
