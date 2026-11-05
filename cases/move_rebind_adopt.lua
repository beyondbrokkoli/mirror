-- EXPECT: 2.5
-- EXPECT: 1
local t = {1}
local u = t
t = {2.5}
print(t[0])
print(u[0])
