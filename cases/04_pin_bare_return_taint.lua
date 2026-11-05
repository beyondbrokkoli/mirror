-- EXPECT_BUILD_FAIL: Lifetime Error: the boundary table 'res' is pinned — the host owns its header; read its cells ('res[i]') instead of moving the table
local f = function(x)
    return x
end
local res = f(arg)
local t = {}
t[1] = res
return t
