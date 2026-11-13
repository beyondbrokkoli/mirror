-- EXPECT_BUILD_FAIL: cyclic row store
-- The one tuple shape the cyclic-row law refuses: storing a[1]'s row
-- into a[0] would make the outer table's element type cycle through
-- its own rows (alm's element types are finite). The scalar twin
-- (tuple_swap_parallel) stays green because ints are values, not rows.
local a = {{9}, {9}}
a[0], a[1] = a[1], a[0]
print(a[0][0], a[1][0])
