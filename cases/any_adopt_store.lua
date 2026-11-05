-- EXPECT: five
-- EXPECT: 3
-- EXPECT: 4
local t = {7}
t[2] = "five"
print(t[2])
print(#t)
print(#t[2])
