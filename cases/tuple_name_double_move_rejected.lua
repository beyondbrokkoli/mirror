-- EXPECT_BUILD_FAIL: 't' is moved more than once
-- The obj latch meets the move law (T2): 't[0], t = 5, t' must latch
-- the obj — t rebinds before the cell store lands — AND latch t as
-- the second value, two moves of one table on one decl. The judge has
-- no move law and runs this clean; the dialect refuses by name, the
-- same law tuple_same_table_rejected pins for pure cell targets.
local t = {0, 0}
t[0], t = 5, t
print(t[0], t[1])
