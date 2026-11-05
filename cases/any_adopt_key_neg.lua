-- EXPECT_BUILD_FAIL: table index must be an Integer or a String, got Any — 't' went dynamic at line
local t = {1}
t[2] = "s"
local u = {}
u[t[0]] = 1
