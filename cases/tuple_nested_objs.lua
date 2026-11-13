-- EXPECT: 5	6
-- Nested-obj targets through the latch (T1): each target's obj
-- evaluates before any store, so the row reads a[0] and a[1] latch
-- into tuple$o temps and both stores ride the latches. The judge
-- agrees (luajit and lua5.4 both print 5 6); distinct cells never
-- observe store order, so this pins the latch, not the order.
local a = { {1, 2}, {3, 4} }
a[0][1], a[1][0] = 5, 6
print(a[0][1], a[1][0])
