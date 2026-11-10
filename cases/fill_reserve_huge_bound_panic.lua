-- EXPECT_PANIC: table allocation
-- The no-offset sibling: the reserve is the raw bound, and a
-- maxinteger bound is unallocatable — the allocator's checked layout
-- math aborts loudly. Pinning the current law: pre-header
-- over-reporting on an unfillable bound dies diagnosed (the extreme
-- data point for the P10 zero-trip decision).
local function fill()
  local t = {}
  local i = 0
  while i < 9223372036854775807 do
    t[i] = i
    i = i + 1
    if i > 8 then return t end
  end
end
local t = fill()
print(#t)
