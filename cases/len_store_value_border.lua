-- EXPECT: 1
local s = {}
s[0] = 7
local t = {}
t[0] = #s
print(t[0])
