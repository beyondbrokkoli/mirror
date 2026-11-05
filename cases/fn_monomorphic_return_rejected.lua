-- EXPECT_BUILD_FAIL: Type Error: type conflict — Integer vs String
local f = function() return 1 end
local g = function() return f() end
print(g())
f = function() return "s" end
print(g())
