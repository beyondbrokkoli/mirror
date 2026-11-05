-- EXPECT: 1
local t = {1}
local u = t
t = nil
print(u[0])
