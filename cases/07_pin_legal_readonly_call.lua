-- ARGS: 41 7
local f = function(x)
    return x[0] + x[1]
end
local res = f(arg)
local t = {}
t[1] = res
return t
