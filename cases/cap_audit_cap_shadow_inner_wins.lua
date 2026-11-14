-- EXPECT: 10
-- EXPECT: 1
-- Audit-mandated probe: the shadow wins at its own depth;
-- judge-matched.
-- Shadowed capture: an inner do-block shadows the captured name; two
-- sibling closures must capture DIFFERENT cells (the shadow for the one
-- created inside, the outer for the one created after).
-- LuaJIT: 10, 1
local function Make()
    local x = 1
    local inner
    do
        local x = 10
        local function GetInner() return x end
        inner = GetInner
    end
    local function GetOuter() return x end
    return { inner = inner, outer = GetOuter }
end
local d = Make()
print(d.inner())
print(d.outer())
