-- EXPECT_BUILD_FAIL: Type Error: a row of 'm' is read before it is ever given a value (table site #0)

local x = {}
local m = {x}
print(m[0][0])
