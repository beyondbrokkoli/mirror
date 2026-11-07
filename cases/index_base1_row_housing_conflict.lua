-- FLAGS: --index-base=1
-- EXPECT_BUILD_FAIL: Lifetime Error: the destination cell already houses a row
-- The row-store conflict lookup reads the store key through the
-- base-aware reader: t's slot map holds physical slots (the ghost-keep
-- writer shifted 1 -> 0), and the second store's key 1 must find it. A
-- missed shift is a silent miss -- the displaced row would leak
-- unnamed -- so this rejection is the law case for the reader at the
-- conflict site.
local a = {{1, 2}, {3}}
local t = {}
t[1] = a[1]
t[1] = a[2]
