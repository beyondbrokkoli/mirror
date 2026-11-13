-- EXPECT_BUILD_FAIL: line 9: Lifetime Error: the boundary table 'arg' is pinned
-- The pinned-arg law through the name-target latch (T2; the archived
-- P3 receipt rides the same seam): 'x, y = arg, x' latches arg into a
-- value temp — a direct move of the boundary table, which the host
-- owns. The rejection lands on the tuple header's line: every node of
-- the expansion carries the header's stamp.
local x = {0}
local y = {1}
x, y = arg, x
print(x[0], y[0])
