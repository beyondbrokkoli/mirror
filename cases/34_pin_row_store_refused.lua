-- EXPECT_BUILD_FAIL: Lifetime Error: a row read with a dynamic or unproven key cannot be stored
local a = {{1, 2}, {3}}
local i = 0
local t = {}
t[0] = a[i]
print(t[0][0])
