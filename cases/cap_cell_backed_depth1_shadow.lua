-- EXPECT: 42
-- EXPECT: 42
-- Phase 4 cell-backed locals, triage Bug 2 — LANDED GREEN. Finding B
-- at depth >= 1: F's view inside M's expansion resolves at the
-- recorded walk depth (Run's x), so M's x=100 shadow cannot re-point
-- the body's reads or writes. Was 140,2 — the d == 0 filters kept
-- deeper captures on dynamic innermost reads.
local function Run()
    local x = 2
    local function F() x = x + 40 return x end
    local function M()
        local x = 100
        return F()
    end
    print(M())
    print(x)
end
Run()
