-- EXPECT: 3
-- EXPECT: 3
-- EXPECT: 2
-- EXPECT: 0
local t = {1, 2, 3}
print(#t)
local u = t
print(#u)
print(#{10, 20})
print(#{})
