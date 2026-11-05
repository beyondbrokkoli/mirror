-- EXPECT: 10
-- EXPECT: 0
local k = function() return 0 end
local g = function()
    local t = {10, 20, 30}
    print(t[k()])
end
g()
k = function() return "x" end
g()
