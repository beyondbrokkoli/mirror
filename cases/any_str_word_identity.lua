-- ARGS: alm/run.lua
-- EXPECT: true
-- EXPECT: false
local t = {0}
t[0] = "alm/run.lua"
t[1] = arg[0]
t[2] = "alm/elsewhere.lua"
print(t[0] == t[1])
print(t[0] == t[2])
return t
