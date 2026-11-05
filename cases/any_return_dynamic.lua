-- EXPECT: grown
local t = {1, "two", 2.5, true}
t[10] = "grown"
print(t[10])
return t
