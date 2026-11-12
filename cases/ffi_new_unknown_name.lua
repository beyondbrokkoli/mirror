-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.new: no declaration named 'nope' — ffi.cdef it first
local ffi = require("ffi")
local t = ffi.new("nope", 4)
