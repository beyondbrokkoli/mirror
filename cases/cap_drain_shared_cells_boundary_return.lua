-- EXPECT: 42
-- Shared cell, boundary return between the envs' readers. The return
-- path's drain scopes its keeps to the prefix: the cell is held by
-- env2's mint only AFTER this return, so the return path frees it
-- outright while the fall-through path keeps-then-frees it through
-- the last holder. Judge: 42 on the taken path.
local n = 41
local function bump() n = n + 1 end
local h = bump
h()
print(n)
if n == 42 then return nil end
print("unreachable")
