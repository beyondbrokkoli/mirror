-- EXPECT: 41
-- ARGS: 41
local f = function(x)
    local u = {}
    u[1] = x[0]
    return u
end
local res = f(arg)
print(res[1])
