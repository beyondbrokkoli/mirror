-- EXPECT: 1
local f = function() return 1 end
local g = f
print(g())
