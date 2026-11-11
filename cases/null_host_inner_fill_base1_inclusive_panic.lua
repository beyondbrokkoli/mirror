-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT_PANIC: alm runtime error: attempt to index a nil value (null table row)
-- The P20 refused-outer quadrant under the inclusive latch: the
-- studied quadrant of the original probe (gdb put the fault on the
-- inner store's header load, one instruction before the gep+store).
-- Same law as the exclusive twin, one cell later in the border
-- arithmetic — the decline is base-driven, not limit-driven: the
-- refusal of t[0] -> physical -1 is identical under both latches, and
-- so is the loud abort on the first inner host load.
local t = {}
for i = 0, 3 do
  t[i] = { i }
  for j = 1, 2 do
    t[i][j] = i * 10 + j
  end
end
print(#t)
print(#t[1])
print(t[1][1])
print(t[3][2])
print(sys_alloc_count())
