-- EXPECT: 0
-- EXPECT: 7
-- EXPECT: 5
-- EXPECT: 3
-- EXPECT: 3
local st = function(t) t[9] = 1 end
st({})
print(sys_alloc_count())
local carrier = {{9}}
local mv = function(x) carrier[1] = x end
mv({7, 8})
print(carrier[1][0])
local br = function(x) return x end
local held = br({5})
print(held[0])
held = nil
print(sys_alloc_count())
br({6})
print(sys_alloc_count())
