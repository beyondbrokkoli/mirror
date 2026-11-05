-- EXPECT: 5	6
local arg = {}
arg[1] = 5
local t = arg
t[2] = 6
print(t[1], t[2])
t = nil
