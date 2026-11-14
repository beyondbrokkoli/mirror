-- EXPECT: 10
-- EXPECT: 11
-- EXPECT: 10
-- Audit-mandated probe: the escaping closure. Two Make()
-- invocations own INDEPENDENT boxes (per-instance slot ids), and
-- the box outlives the definer frame. alm matched pre-fix; the
-- shared-cell redesign had to preserve it (10, 11, 10, never 12).
-- Escaping closure: the captured scalar's home frame is gone when the
-- closure runs; the box must keep the value alive (lifetime extension),
-- and two independent Make() invocations must not share cells.
-- LuaJIT: 10, 11, 10
local function Make(seed)
    local n = seed
    local function Step() n = n + 1 return n end
    return { step = Step }
end
local a = Make(9)
local b = Make(9)
print(a.step())
print(a.step())
print(b.step())
