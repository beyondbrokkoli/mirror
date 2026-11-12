-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.sizeof: no declaration named 'ghost' — ffi.cdef it first
local ffi = require("ffi")
print(ffi.sizeof("ghost"))
