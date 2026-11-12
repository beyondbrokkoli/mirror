-- FLAGS: --exe
-- EXPECT_BUILD_FAIL: ffi.cdef cannot parse
local ffi = require("ffi")
ffi.cdef("int +++ cell;")
