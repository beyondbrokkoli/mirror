-- EXPECT: 42
-- EXPECT: 42
-- Closure Finding B, blessed-record surface — LANDED GREEN. The
-- record of closures is created at TOP level; the creation read now
-- mints a face that carries x (depth 0 is in the capture chain), so
-- the field dispatch inside M expands Get against the shared cell,
-- not M's x=100. Retired pin — was EXPECT_BUILD_FAIL: (Closure
-- Finding B) back when the empty face made x resolve to the shadow
-- (alm printed 140, 2).
-- LuaJIT: 42, 42 · alm pre-tripwire: 140, 2
local x = 2
local function Get() x = x + 40 return x end
local d = { Get = Get }
local function M()
    local x = 100
    return d.Get()
end
print(M())
print(x)
