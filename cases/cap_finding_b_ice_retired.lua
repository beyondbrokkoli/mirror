-- EXPECT: 42
-- EXPECT: 42
-- The receipt that upgraded Finding B from "law surface" to
-- "internal desync" — LANDED GREEN. Pre-tripwire this program
-- PANICKED the compiler at src/lowerer/bridges.rs:75 (the repr law):
-- the checker resolved x from depth-0 top (Integer) while the
-- lowerer read M's Str register. Now the face carries x from the
-- mint (resolved depth-aware at the creation site), the expansion's
-- face local rides the shared cell on both sides, and the shadow of
-- a DIFFERENT type cannot split checker from lowerer. Retired pin —
-- was EXPECT_BUILD_FAIL: (Closure Finding B) (the tripwire retired
-- the ICE before this fix did).
-- LuaJIT: 42, 42 · alm pre-tripwire: ICE (bridges.rs:75)
local x = 2
local function Get() x = x + 40 return x end
local d = { Get = Get }
local function M()
    local x = "hi"
    return d.Get()
end
print(M())
print(x)
