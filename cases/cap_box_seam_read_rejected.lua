-- EXPECT: 42
-- Box-identity seam, read side — LANDED GREEN. The closure's
-- expansion writes the shared cell and the pop fold-back re-reads it
-- into the definer's register (the slot located exactly by the
-- mint-time scope id), so `return n` sees the callee's write. Was
-- Was a BUILD_FAIL pin (Closure box-identity seam) back when the
-- definer's register went stale (alm printed 41).
-- LuaJIT: 42 · alm pre-shame: 41
local function Make()
    local n = 41
    local function Bump() n = n + 1 return n end
    local b = Bump
    b()
    return n
end
print(Make())
