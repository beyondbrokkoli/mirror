-- ARGS: 7
-- EXPECT: 1
-- EXPECT: 7
-- EXPECT: 2
-- EXPECT: 1
print(sys_alloc_count())
print(arg[0])
local t = {1, 2}
print(sys_alloc_count())
t = nil
print(sys_alloc_count())
return arg
