-- FLAGS: --index-base=1
-- EXPECT: 1
-- EXPECT: 0
-- EXPECT: 2
-- XFAIL: judge_divergence_audit/reserve-border — #t reports the
-- reserve (the shifted sparse threshold stays dense); the judge
-- reports the border (0 — t[1] is nil). Flip with the # border law
-- (len_runtime_border, len_sparse_ctor_border).
-- The sparse/bounds thresholds compare physical values: user key
-- 100001 shifts to physical 100000, exactly SPARSE_THRESHOLD, which
-- stays dense -- a reader missing the shift would join the sparse
-- verdict and the border would not read 100001. The offset fill
-- (t[i + 1], physical offset 0) then rides the lane and lands 2 at
-- user key 3.
local t = {}
t[50000*2+1] = 1
print(t[100001])
print(#t)
local i = 0
while i < 3 do
  t[i + 1] = i
  i = i + 1
end
print(t[3])
t = nil
