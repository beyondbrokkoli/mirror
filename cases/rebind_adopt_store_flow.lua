-- EXPECT: 2.5
local t = {}
t[0] = 1
t = {}
t[0] = 2.5
print(t[0])
