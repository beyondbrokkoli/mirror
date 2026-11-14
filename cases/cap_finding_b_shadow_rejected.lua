-- EXPECT: 42
-- EXPECT: 42
-- Closure Finding B, direct-call surface — LANDED GREEN. F reads
-- top-level x; the capture chain now includes depth 0 (the skip(1)
-- retired), so F captures x and the direct expansion declares a
-- capture view of the root: M's x=100 cannot re-point the body's
-- reads or writes. Retired pin — was EXPECT_BUILD_FAIL: (Closure
-- Finding B) back when top-level fn literals captured nothing and
-- the expansion resolved x dynamically (alm printed 140, 2).
-- LuaJIT: 42, 42 · alm pre-tripwire: 140, 2
local x = 2
local function F()
    x = x + 40
    return x
end
local function M()
    local x = 100
    return F()
end
print(M())
print(x)
