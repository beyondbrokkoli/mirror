-- EXPECT: 42
-- EXPECT: 42
-- Audit-mandated probe: same statement, same face, one shared
-- cell; judge-matched.
-- Two closures minted in the SAME statement capturing the SAME binding.
-- LuaJIT: 42, 42.
local function Make()
    local n = 41
    local function Bump() n = n + 1 return n end
    local function Peek() return n end
    local rec = { bump = Bump, peek = Peek }
    return rec
end
local d = Make()
print(d.bump())
print(d.peek())
