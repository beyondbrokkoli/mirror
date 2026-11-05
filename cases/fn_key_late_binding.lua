-- EXPECT: 1
-- EXPECT: 2
local f = function() return 1 end
local g = function() return f() end
print(g())
f = function() return 2 end
print(g())
