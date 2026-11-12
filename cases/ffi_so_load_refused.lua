-- M4: loading stays --exe-only. The acyclicity law survives as
-- link-time accounting: a .so may ride the callee seam, but it may
-- never itself load a shared object.
-- EXPECT_BUILD_FAIL: alm compiler error: ffi.load is --exe-only — a shared object may not load shared objects
local ffi = require("ffi")
local lib = ffi.load("libm.so")
