-- EXPECT: 8
-- EXPECT: 0
local m = {}
m[0] = {7, 8}
print(m[0][1])
m = nil
print(sys_alloc_count())
