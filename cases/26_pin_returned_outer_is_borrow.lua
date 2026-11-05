-- EXPECT: 1
-- EXPECT: 2
local t = {1, 2}
local f = function() return t end
local x = f()
print(x[0])
x = nil
print(t[1])
