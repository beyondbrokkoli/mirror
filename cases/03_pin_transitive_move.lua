-- EXPECT_BUILD_FAIL: Lifetime Error: 'f' moves its parameter 'x' into another table or binding — the pinned table 'arg' may not be passed to it (the host owns the boundary header)
local f = function(x)
    local t = {}
    t[1] = x
    return t
end
local res = f(arg)
return res
