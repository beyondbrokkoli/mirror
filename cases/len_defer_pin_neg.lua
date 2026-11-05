-- ARGS: x
-- EXPECT_BUILD_FAIL: '#' requires a Table, String, or Any operand, got Integer
print(#arg[0])
local n = arg[0] + 1
