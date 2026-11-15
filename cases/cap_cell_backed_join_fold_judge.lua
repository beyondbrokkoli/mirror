-- EXPECT: 42
-- EXPECT: 42
-- Phase 4 cell-backed locals, the triage VOID claims pinned as cases
-- (Bug 3a's dynamic variants): the join/phi machinery carries cell
-- writes across if/else and while joins — memory is the merge, no
-- fold-back needed. The calls ride the arm and the loop body (V1
-- bans creations there, not calls — Bug 3b's void); the cell write
-- lands in the join's read either way.
local x = 2
local function F() x = x + 40 end
if true then F() else x = 0 end
print(x)
x = 2
local cond = false
while cond == false do
    cond = true
    F()
end
print(x)
