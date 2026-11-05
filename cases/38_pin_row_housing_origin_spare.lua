-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 0
local a = {{1, 2}, {3}}
local t = {}
t[0] = a[0]
a = nil
print(sys_alloc_count())
print(t[0][0])
print(t[0][1])
t = nil
print(sys_alloc_count())
