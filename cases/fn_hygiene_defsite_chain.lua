-- EXPECT: 10
local f = function() return 1 end
do
    local f = function() return 10 end
    local g = function() return f() end
    print(g())
end
