-- FLAGS: --index-base=1
-- EXPECT_BUILD_FAIL: Lifetime Error: cyclic row store
-- The ghost's birth site resolves through the physical slot map: the
-- outer ctor's positional entry minted slot 0, and the read key 1 must
-- shift to 0 to find it. A missed shift turns the cycle rejection into
-- the wrong rejection (birth site not visible), so the exact message
-- is the pin.
local a = {{{5, 6}}}
a[1][1] = a[1]
