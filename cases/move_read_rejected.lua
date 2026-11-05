-- EXPECT_BUILD_FAIL: Lifetime Error: 't' may be moved here

local t = {1}
local u = t
print(t[0])
