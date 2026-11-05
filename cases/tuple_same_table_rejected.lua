-- EXPECT_BUILD_FAIL: 't' is moved more than once
local t = {7}
local a = {{9}, {9}}
a[0], a[1] = t, t
