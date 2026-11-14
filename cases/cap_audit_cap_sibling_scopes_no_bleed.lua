-- EXPECT: 15
-- EXPECT: 15
-- EXPECT: 21
-- EXPECT: 10
-- Audit-mandated probe: sibling scopes never bleed; judge-matched.
-- Sibling do-blocks declare the same name x and a closure over their
-- own shadow; each call must mutate ITS OWN shadow, and nothing may
-- bleed across the sibling scopes.
-- LuaJIT: 15, 15, 21, 10
local outer = 10
do
    local x = outer
    local function GA() x = x + 5 return x end
    print(GA())
    print(x)
end
do
    local x = 20
    local function GB() x = x + 1 return x end
    print(GB())
end
print(outer)
