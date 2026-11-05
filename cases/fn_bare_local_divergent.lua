-- EXPECT: 1
-- EXPECT: s
local f = function() return 1 end
local g = function()
    local x
    x = f()
    print(x)
end
g()
f = function() return "s" end
g()
