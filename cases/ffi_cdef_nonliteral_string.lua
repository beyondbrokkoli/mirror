-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.cdef's declaration is a compile-time string literal
local ffi = require("ffi")
local s = "int cell;"
ffi.cdef(s)
