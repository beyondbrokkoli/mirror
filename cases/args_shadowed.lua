-- ARGS: ignored words
-- EXPECT: 3
-- EXPECT: 0
local arg = 3
print(arg)
print(sys_alloc_count())
