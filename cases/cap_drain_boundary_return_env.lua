-- EXPECT: 42
-- The boundary return. The root drain rides the fall-through tail
-- only; a chunk leaving through a top-level return carries its own —
-- the creation-read env and its held cell both leaked pre-fix
-- (leak_contract/p8). Judge: 42.
local n = 41
local function bump() n = n + 1 end
local h = bump
h()
print(n)
return nil
