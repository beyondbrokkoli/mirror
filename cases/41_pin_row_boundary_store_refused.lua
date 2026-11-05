-- EXPECT_BUILD_FAIL: Lifetime Error: a row read cannot be stored into an unresolved or host-owned table
-- ARGS: 1
local a = {{1, 2}}
arg[0] = a[0]
