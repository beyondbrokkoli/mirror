-- EXPECT_BUILD_FAIL: Type Error: 'f' is an expansion with no return — it produces no value
local f = function() end
local x = f()
print(x)
