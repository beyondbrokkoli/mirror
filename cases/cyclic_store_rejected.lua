-- EXPECT_BUILD_FAIL: Type Error: cyclic table store — 't' stored into 't' would create a table cycle (alm's element types are finite)

local t = {}
t[0] = t
