-- ARGS: alpha beta
-- EXPECT: 3
-- EXPECT: 69999
-- EXPECT: 0
-- EXPECT: 2
-- XFAIL: judge_divergence_audit/reserve-border — #t reports the
-- reserve (the raw bound, sparse keys included); the judge reports
-- the border (69999 on the contiguous fill, 0 on the lone sparse
-- key). Line 4 (#arg) already matches the judge. Flip with the
-- # border law.
local t = {1, 2, 3}
print(#t)
local i = 3
while i < 70000 do
  t[i] = i
  i = i + 1
end
print(#t)
local s = {}
s[200000] = 7
print(#s)
print(#arg)
return t
