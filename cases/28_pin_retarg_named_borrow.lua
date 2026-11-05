-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 7
-- EXPECT: 1
local t = {1, 2}
local g = function(x) return x end
local y = g(t)
print(y[0])
y = nil
print(t[1])
print(sys_alloc_count())
local r = g({7, 8})
print(r[0])
r = nil
print(sys_alloc_count())
