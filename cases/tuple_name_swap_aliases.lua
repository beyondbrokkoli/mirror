-- EXPECT: 2	1
-- The alias swap through the sugar (T2): each table moves exactly
-- once into its value temp and each rebind lands after the latch —
-- the composition tuple_latch_swap_aliases hand-writes, now spelled
-- the way source does. The judge prints 2 1.
local x = {1}
local y = {2}
x, y = y, x
print(x[0], y[0])
