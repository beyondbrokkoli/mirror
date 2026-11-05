-- EXPECT_BUILD_FAIL: a table value never rides a cell
local t = {1, "s"}
t[0] = {}
