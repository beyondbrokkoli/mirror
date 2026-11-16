-- EXPECT: 0
-- XFAIL: judge_divergence_audit/reserve-border — #s reports the
-- reserve (1 for the key-0 store); the judge reports the border (0),
-- so t[0] answers 0. Flip with the # border law.
local s = {}
s[0] = 7
local t = {}
t[0] = #s
print(t[0])
