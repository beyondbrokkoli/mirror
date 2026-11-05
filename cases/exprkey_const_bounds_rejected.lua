-- EXPECT_BUILD_FAIL: Table Bounds Error: table index overflow
local t = {}
t[1152921504606846975 + 1] = 1
