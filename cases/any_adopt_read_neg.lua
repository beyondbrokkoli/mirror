-- EXPECT_BUILD_FAIL: '+' requires numeric operands — 't' went dynamic at line
local t = {1}
t[2] = "two"
print(t[0] + 1)
