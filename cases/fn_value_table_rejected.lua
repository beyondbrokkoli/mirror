-- EXPECT_BUILD_FAIL: Type Error: a closure value enters only a record's literal fields
local f = function() return 1 end
local t = {}
t["x"] = f
