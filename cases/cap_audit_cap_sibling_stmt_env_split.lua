-- EXPECT: 42
-- EXPECT: 42
-- Audit-mandated probe: the env split across a statement seam;
-- judge-matched.
-- Sibling closures created at DIFFERENT statements, same captured
-- scalar: mutations must be visible across both (LuaJIT upvalues are
-- one shared cell).
-- LuaJIT: 42, 42
local function Make()
    local n = 41
    local function Bump() n = n + 1 return n end
    local function Peek() return n end
    return { bump = Bump, peek = Peek }
end
local d = Make()
print(d.bump())
print(d.peek())
