-- M4: the .so lane is open to the callee seam. cdef + ffi.new +
-- ffi.sizeof with no ffi.load compiles green without --exe — the
-- module exposes alm_arg_kind/alm_exec and never loads anything, so
-- the acyclicity law holds by construction, not by a blanket ban.
-- EXPECT: 100
-- EXPECT: 1.5	2.5
-- EXPECT: 800
local ffi = require("ffi")
ffi.cdef("double[100];")
local t = ffi.new("double[100]")
print(#t)
t[0] = 1.5
t[1] = 2.5
print(t[0], t[1])
print(ffi.sizeof("double[100]"))
