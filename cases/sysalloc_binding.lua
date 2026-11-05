-- EXPECT: 0
-- EXPECT: 2

local z = sys_alloc_count()
print(z)

local t = {}
t[0] = 1
local u = {}
u[0] = 2
local c = sys_alloc_count()
print(c)
