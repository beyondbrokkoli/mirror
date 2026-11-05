-- EXPECT_BUILD_FAIL: a table value never rides a cell
local t = {1, {2}}
print(t[0])
