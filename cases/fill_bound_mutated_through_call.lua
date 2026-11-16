-- EXPECT: 0
-- XFAIL: judge_divergence_audit/reserve-border — the reserve latches
-- the entry bound (b = 3) and #t reports it after Shrink() halves
-- the loop; the judge reports the border (0 — only t[0] was
-- written). Flip with the # border law.
local t = {}
local b = 3
local n = 0
local function Shrink()
    b = 1
    return 0
end
while n < b do
    t[n] = n * 2
    n = n + 1
    Shrink()
end
print(#t)
