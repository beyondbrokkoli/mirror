-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 1
local f = function() return 1 end
local g = function() return f() end
local s = function()
    local f = function() return 10 end
    return g()
end
print(g())
print(s())
print(g())
