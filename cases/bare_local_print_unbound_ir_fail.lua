-- EXPECT_BUILD_FAIL: Lifetime Error: 'x' may be nil here — reads through a possibly-nil name are rejected at compile time
local x
print(x)
