-- FLAGS: --exe
-- ffi.sizeof is the registry's cheapest projection: bytes_of is the
-- one law — capacity*esize when the declaration carries a capacity,
-- the elem's esize otherwise.
-- EXPECT: 800
-- EXPECT: 8
-- EXPECT: 1
local ffi = require("ffi")
ffi.cdef("double[100];")
ffi.cdef("int unit;")
ffi.cdef("bool flag;")
print(ffi.sizeof("double[100]"))
print(ffi.sizeof("unit"))
print(ffi.sizeof("flag"))
