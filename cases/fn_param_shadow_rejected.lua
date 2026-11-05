-- EXPECT_BUILD_FAIL: Type Error: 'f' is not an expansion — a call through a value needs a closure
local f = function() return 1 end
local apply = function(f) return f() end
print(apply(5))
