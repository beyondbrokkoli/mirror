-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.new's capacity is an integer literal
local ffi = require("ffi")
ffi.cdef("int cell;")
local n = 4
local t = ffi.new("cell", n)
