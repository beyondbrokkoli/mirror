-- EXPECT: 2.5
local t = {1}
t = nil
t = {}
t[0] = 2.5
print(t[0])
