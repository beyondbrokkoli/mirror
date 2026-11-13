-- EXPECT: 4	1
-- Rows swap through names where cells refuse (T2): 'x, y = y, x' over
-- two row reads rebinds the borrows — no cell ever stores a row, so
-- the cyclic-row law that stops tuple_row_swap_rejected has nothing
-- to refuse. The judge prints 4 1.
local a = {{1, 2}, {3, 4}}
local x = a[0]
local y = a[1]
x, y = y, x
print(x[1], y[0])
