-- EXPECT_BUILD_FAIL: Syntax Error: duplicate index 0
local t = {10, [0] = 5}
print(t[0])
