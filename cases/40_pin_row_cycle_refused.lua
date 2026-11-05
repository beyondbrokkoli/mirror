-- EXPECT_BUILD_FAIL: Lifetime Error: cyclic row store
local a = {{{5, 6}}}
a[0][0] = a[0]
