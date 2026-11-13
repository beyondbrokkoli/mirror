-- EXPECT: 2	3	1
-- The n-cell rotate is the tuple form's reason to exist: every RHS
-- latches into a temp before any store, so a rotation reads the old
-- cells even as it overwrites them. Matches the judge (LuaJIT prints
-- 2 3 1) and stays green through the T1 store-order flip — distinct
-- targets never observe store order.
local t = {1, 2, 3}
t[0], t[1], t[2] = t[1], t[2], t[0]
print(t[0], t[1], t[2])
