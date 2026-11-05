-- EXPECT: 1
-- EXPECT: x
-- EXPECT: 5
-- EXPECT: 2
local t = { {1}, {"x"} }
print(t[0][0])
print(t[1][0])
t[0][0] = 5
print(t[0][0])
print(#t)
