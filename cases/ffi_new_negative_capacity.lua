-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.new's capacity is a positive integer literal
local ffi = require("ffi")
ffi.cdef("int cell;")
local t = ffi.new("cell", -3)
