-- EXPECT_BUILD_FAIL: Type Error: cannot index a row of 'm' — no table is ever stored into 'm', and stores do not create rows (table site #0)
local m = {}
m[0][0] = 5
