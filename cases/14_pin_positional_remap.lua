-- EXPECT_BUILD_FAIL: Lifetime Error: the boundary table 'res' is pinned — the host owns its header; read its cells ('res[i]') instead of moving the table
local g = function(y)
    return y
end

local f = function(x, y)
    return g(y)
end

local res = f({}, arg)
local t = {}
t[1] = res
t = nil
