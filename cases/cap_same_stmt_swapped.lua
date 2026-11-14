-- Closure Finding A pin, swapped field order: GetOuter's creation
-- read comes FIRST in the ctor. The old memo was order-dependent only
-- in which face won; both orders printed 10, 10. Lexical capture is
-- order-independent — each face stores its own binding at its own
-- walk-scope depth.
-- LuaJIT: 10, 1
-- EXPECT: 10
-- EXPECT: 1
local function Make()
    local x = 1
    local function GetOuter() return x end
    local rec
    do
        local x = 10
        local function GetInner() return x end
        rec = { b = GetOuter, a = GetInner }
    end
    return rec
end
local d = Make()
print(d.a())
print(d.b())
