-- EXPECT: 1
-- EXPECT: 2
-- Audit-mandated probe: disjoint faces share one env; judge-matched.
-- Sibling closures minted in ONE statement with DISJOINT capture faces.
-- LuaJIT: 1, 2
local function Make()
    local a = 1
    local b = 2
    local function GA() return a end
    local function GB() return b end
    return { ga = GA, gb = GB }
end
local d = Make()
print(d.ga())
print(d.gb())
