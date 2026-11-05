-- EXPECT_BUILD_FAIL: Lifetime Error: the boundary table 'arg' is pinned — the host owns its header; read its cells ('arg[i]') instead of moving the table
local u = {}
u = arg
print(u[0])
