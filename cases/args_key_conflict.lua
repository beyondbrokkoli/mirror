-- EXPECT_BUILD_FAIL: Type Error: table index must be an Integer or a String, got Boolean
print(arg[0] == true)
local t = {}
t[arg[0]] = 1
