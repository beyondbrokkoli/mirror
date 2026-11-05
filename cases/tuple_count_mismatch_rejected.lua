-- EXPECT_BUILD_FAIL: tuple assignment binds 3 targets to 2 values — counts must match
local t = {0, 0, 0}
t[0], t[1], t[2] = 1, 2
