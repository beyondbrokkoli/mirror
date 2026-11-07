-- FLAGS: --index-base=1
-- EXPECT_BUILD_FAIL: Lifetime Error: this row was already released with its housing
-- The claimed-read slot check compares the read's key against the
-- physical slot map; key 1 must shift to 0 or the claim lookup misses
-- and the read of the released row would pass silently.
local a = {{1, 2}, {3}}
local t = {}
t[1] = a[1]
t = nil
print(a[1][1])
