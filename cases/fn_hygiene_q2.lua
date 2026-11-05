-- EXPECT: 2
local f = function() return 1 end
local g = function() return f() + f() end
local h = function()
    local f = function() return 42 end
    return g()
end
print(h())
