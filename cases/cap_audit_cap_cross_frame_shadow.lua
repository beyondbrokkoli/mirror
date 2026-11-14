-- EXPECT: 42
-- EXPECT: 42
-- Audit-mandated probe, cornered 2026-11-14: a top-level fn's body
-- reads top-level x across two call frames; pre-Finding-B-fix the
-- dynamic read re-pointed to a caller shadow (alm 140, 2). The
-- capture chain now includes depth 0.
-- The deep chain is fully inline-expanded; when L1 is called from a
-- frame that shadows x, the expanded body must still see the
-- DEFINER's x (root), not the caller's.
-- LuaJIT: 42, 42
local x = 2
local function L1()
    local function L2()
        local function L3()
            x = x + 40
            return x
        end
        return L3
    end
    return L2
end
local function M()
    local x = 100
    local c2 = L1()
    local c3 = c2()
    return c3()
end
print(M())
print(x)
