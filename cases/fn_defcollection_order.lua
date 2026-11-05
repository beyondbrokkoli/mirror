-- EXPECT: 2
-- EXPECT: 2
local f = function() return 1 end
local g = function() f = function() return 2 end end
print(f())
g()
print(f())
