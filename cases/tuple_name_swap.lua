-- EXPECT: 2	1
-- EXPECT: 2
-- Customer #1 (T2): name targets ride the tuple latch. 'x, y = y, x'
-- desugars into the composition tuple_latch_swap_scalars hand-writes
-- — the values latch into tuple$t temps, then the rebinds land
-- right-to-left — and the judge agrees (luajit and lua5.4 both print
-- 2 1). The second line swaps function parameters: a name target is
-- any declared binding, at any depth.
local x = 1
local y = 2
x, y = y, x
print(x, y)
function swap_read(a, b)
  a, b = b, a
  return a
end
print(swap_read(1, 2))
