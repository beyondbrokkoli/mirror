-- EXPECT_BUILD_FAIL: Type Error: a row of 'm' is read before it is ever given a value (table site #2)

local m = {{{}}}
print(m[0][0][0])
