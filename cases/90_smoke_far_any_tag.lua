-- ARGS: 42 world
-- EXPECT: 42
-- EXPECT: world
-- EXPECT: world
local t = {arg[0], arg[1]}
t[200000] = arg[1]
print(t[0])
print(t[200000])
print(arg[1])
return t
