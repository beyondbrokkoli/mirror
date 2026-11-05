-- EXPECT: 7
-- EXPECT: 7


local t = {{7, 8}}
local x = t[0]
t = {{5}}
print(x[0])

local u = {{7, 8}}
local y = u[0]
u = nil
print(y[0])
