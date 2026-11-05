-- ARGS: 7
-- EXPECT: 7
-- EXPECT: 9
local u = {arg[0]}
print(u[0])
local t = {}
t[0] = arg[0]
print(t[0] + 2)
