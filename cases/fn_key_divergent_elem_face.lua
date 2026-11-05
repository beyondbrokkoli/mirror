-- EXPECT: 10
-- EXPECT: s
local f = function() return {10, 20} end
local g = function()
    local t = f()
    print(t[0])
end
g()
f = function() return {"s"} end
g()
