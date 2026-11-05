-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 0
print(sys_alloc_count())
local t = {1, 2}
print(sys_alloc_count())
t = nil
print(sys_alloc_count())
