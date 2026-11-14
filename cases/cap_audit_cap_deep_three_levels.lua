-- EXPECT: 42
-- EXPECT: 42
-- Audit-mandated probe: the three-deep call chain writes the
-- top-level x; the write reaches the shared cell and every
-- fold-back, so the top-level print reads the judge's answer.
-- The receipts flagged this shape as the regression skip(1)-drop
-- alone would cause (2, 42 without box-is-binding).
-- Deep capture: closure nested 3 levels deep mutating a top-level local.
-- LuaJIT: 42, 42 (the mutation is visible to the outer frame through the
-- shared upvalue cell).
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
local c2 = L1()
local c3 = c2()
print(c3())
print(x)
