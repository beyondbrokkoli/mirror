-- EXPECT: 51
-- EXPECT: 52
-- EXPECT: 52
-- Box-identity seam t9: the SAME literal called direct and
-- through its value, definer writes between. Pre-fix the two
-- surfaces answered differently (51, 52, 51 — 'direct call reads
-- the register, through-value call reads the cell'). The shared
-- cell and the fold-backs give one answer everywhere.
-- same literal, direct call vs through-value call, definer writes between
local function Make()
    local n = 41
    local function Bump() n = n + 1 return n end
    local b = Bump
    n = 50
    local first = Bump()
    local viaVal = b()
    return { first = first, viaVal = viaVal, reg = n }
end
local d = Make()
print(d.first)
print(d.viaVal)
print(d.reg)
