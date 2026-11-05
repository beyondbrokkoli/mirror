-- EXPECT: 2.5
local t = {{1}}
t = nil
t = {{2.5}}
print(t[0][0])
