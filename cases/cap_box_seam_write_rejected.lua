-- EXPECT: 101
-- EXPECT: 101
-- Box-identity seam, write side — LANDED GREEN. The judge boxes ONCE
-- per binding and every access rides the one cell. The lowerer now
-- shares one box across every env minted over the binding (first
-- mint snapshots, later mints store the same cell pointer), and the
-- definer's `n = 100` routes to it — Bump and Peek answer from one
-- storage. Retired pin — was EXPECT_BUILD_FAIL: (Closure box-identity
-- seam ...) back when the box snapshotted per mint and the definer's
-- write reached only the register (alm printed 42, 100).
-- LuaJIT: 101, 101 · alm pre-shame: 42, 100
local function Make()
    local n = 41
    local function Bump() n = n + 1 return n end
    local b = Bump
    n = 100
    local function Peek() return n end
    local p = Peek
    return { Bump = b, Peek = p }
end
local d = Make()
print(d.Bump())
print(d.Peek())
