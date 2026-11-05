-- EXPECT_BUILD_FAIL: Invalid assignment target
local a = 1
a + 1 = 2
print(a)
