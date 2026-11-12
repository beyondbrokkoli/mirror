-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.new over 'point': flat scalar arrays first — 'point' is not a scalar kind
local ffi = require("ffi")
ffi.cdef("struct point { int x; int y; };")
local t = ffi.new("point", 4)
