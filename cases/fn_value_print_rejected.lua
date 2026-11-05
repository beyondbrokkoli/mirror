-- EXPECT_BUILD_FAIL: Type Error: cannot print a closure
local f = function() return 1 end
print(f)
