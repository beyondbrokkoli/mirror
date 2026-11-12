-- FLAGS: --exe
-- The p11 graduation: cdef -> new -> drive-shaped read. The mint
-- reserves exactly once (one sys_alloc_count rise at the ffi.new),
-- the declared capacity is visible in #t from birth (the P10 law: a
-- reserve is a promise of housing), and the buffer frees clean at
-- root-chunk exit (no slot-90 leak). ffi.sizeof rides the same
-- registry row: bytes_of(capacity, esize).
-- EXPECT: 100
-- EXPECT: 1.5	2.5
-- EXPECT: 800
-- EXPECT: 1
local ffi = require("ffi")
ffi.cdef("double[100];")
local t = ffi.new("double[100]")
print(#t)
t[1] = 1.5
t[50] = 2.5
print(t[1], t[50])
print(ffi.sizeof("double[100]"))
print(sys_alloc_count())
