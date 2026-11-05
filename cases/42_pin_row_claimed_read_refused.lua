-- EXPECT_BUILD_FAIL: Lifetime Error: this row was already released with its housing
local a = {{1, 2}, {3}}
local t = {}
t[0] = a[0]
t = nil
print(a[0][0])
