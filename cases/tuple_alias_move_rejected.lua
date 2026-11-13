-- EXPECT_BUILD_FAIL: may be moved here
-- Two names can never feed two tuple targets: aliasing t through u
-- does not slip past the move law — once the latch reads t, the read
-- through u is a read through a possibly-moved name. The law keys on
-- names, and that is enough here.
local t = {7}
local u = t
local a = {{9}, {9}}
a[0], a[1] = t, u
print(a[0][0], a[1][0])
