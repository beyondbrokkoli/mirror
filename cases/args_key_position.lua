-- ARGS: 2
-- EXPECT: 1
-- EXPECT: 3
-- EXPECT: 30
local t = {}
t[arg[0]] = 1
print(t[2])
local u = {[arg[0]] = 3}
print(u[2])
print({10, 20, 30}[arg[0]])
