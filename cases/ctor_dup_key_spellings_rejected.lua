-- EXPECT_BUILD_FAIL: Syntax Error: duplicate key 'x' in constructor
local t = { x = 1, ["x"] = 2 }
print("unreached")
