-- EXPECT: 42
-- EXPECT: 42
-- Audit-mandated probe: the no-shadow cross-frame chain (the
-- accidental-match tier, now capture-correct by construction);
-- judge-matched.
-- Same chain, but the caller M has NO x at all. If the macro body
-- resolves free names dynamically, the call site must fail to compile
-- (or resolve to nothing). LuaJIT: 42, 42.
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
    local y = 7
    local c2 = L1()
    local c3 = c2()
    return c3()
end
print(M())
print(x)
