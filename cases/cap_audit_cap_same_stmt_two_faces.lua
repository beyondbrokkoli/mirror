-- EXPECT: 10
-- EXPECT: 10
-- Audit-mandated probe: two same-name faces minted in one
-- statement (Closure Finding A's surface), judge-matched.
-- Two closures minted in the SAME statement whose same-name captures
-- resolve to DIFFERENT bindings (the shadow). LuaJIT: 10, 1.
local function Make()
    local x = 1
    local rec
    do
        local x = 10
        local function GetInner() return x end
        local function GetOuter() return x end
        rec = { a = GetInner, b = GetOuter }
    end
    return rec
end
local d = Make()
print(d.a())
print(d.b())
