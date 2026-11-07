-- FLAGS: --index-base=1
-- EXPECT: 1
-- EXPECT: 100001
-- EXPECT: 2
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
