-- EXPECT: 1000
-- EXPECT: 2000
local m = {{}}
m[0][0] = 1000
print(m[0][0])

local g = {}
g[0] = {}
g[0][1] = 2000
print(g[0][1])
