-- EXPECT_BUILD_FAIL: Lifetime Error: 'f' moves its parameter 'x' into another table or binding — the pinned table 'arg' may not be passed to it (the host owns the boundary header)
local h = function(y)
    local u = {}
    u[1] = y
    print(u[1][0])
end

local f = function(x)
    h(x)
end

f(arg)
