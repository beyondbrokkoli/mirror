-- EXPECT: 2
-- EXPECT: 0
local a = {{1, 2}, {3}}
local t = {}
t[0] = a[0]
t = nil
print(sys_alloc_count())
a = nil
print(sys_alloc_count())
