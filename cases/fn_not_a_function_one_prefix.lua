-- EXPECT_BUILD_FAIL: Type Error: 'x' is not an expansion — a call through a value needs a closure
local x = 5
x()
