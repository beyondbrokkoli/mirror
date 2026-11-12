-- FLAGS: --exe
-- The second-writer contract: a conversion loop over a name the mint
-- already reserved emits NO second TableReserve at the loop's
-- pre-header seam (slot 186 pairs with 185 on the same name). The
-- loop's stores ride the mint's reserve: still exactly one
-- allocation, #t is the declared capacity.
-- EXPECT: 16	1.5	1.5
-- EXPECT: 1
local ffi = require("ffi")
ffi.cdef("float[16];")
local t = ffi.new("float[16]")
local n = 16
local i = 1
while i < n do
  t[i] = 1.5
  i = i + 1
end
print(#t, t[1], t[15])
print(sys_alloc_count())
