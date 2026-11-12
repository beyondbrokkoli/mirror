-- FLAGS: --exe
-- Escape-by-move: the minted buffer returned from its scope frees
-- exactly once in the caller — the mint is an ordinary owner-binding,
-- the settled ownership laws apply verbatim (ghost/keep composition,
-- the existing oracle; no slot-90 leak at exit).
-- EXPECT: 7
-- EXPECT: 1
local ffi = require("ffi")
ffi.cdef("int cell[8];")
local function make()
  local t = ffi.new("cell")
  t[1] = 7
  return t
end
local b = make()
print(b[1])
print(sys_alloc_count())
