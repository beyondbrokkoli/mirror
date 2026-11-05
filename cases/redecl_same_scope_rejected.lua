-- EXPECT_BUILD_FAIL: already declared in this scope
local x = 1
local x = 2
print(x)
