-- EXPECT_BUILD_FAIL: cannot tuple-assign the name 'x'
local x = 1
local y = 2
x, y = y, x
