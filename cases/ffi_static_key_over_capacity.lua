-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: 't' is a declared buffer of capacity 4
local ffi = require("ffi")
ffi.cdef("int cell[4];")
local t = ffi.new("cell")
t[9] = 1
