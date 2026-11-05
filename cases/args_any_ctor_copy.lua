-- ARGS: 42 world
-- EXPECT: 42
-- EXPECT: world
local t = {arg[0], arg[1]}
print(t[0])
print(t[1])
return t
