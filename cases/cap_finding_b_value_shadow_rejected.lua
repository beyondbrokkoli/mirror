-- EXPECT: 42
-- EXPECT: 42
-- Closure Finding B, through-value surface — LANDED GREEN. The
-- closure VALUE is called inside a frame that shadows its depth-0
-- capture: the env face carries x (the chain includes depth 0), so
-- enter_env_ctx declares the face local from the shared cell and the
-- shadow never re-points anything. Retired pin — was
-- Was a BUILD_FAIL pin (Closure Finding B) back when the face was
-- empty and the expansion read M's x=100 (alm printed 140, 2).
-- LuaJIT: 42, 42 · alm pre-tripwire: 140, 2
local x = 2
local function Make()
    local function Get()
        x = x + 40
        return x
    end
    return Get
end
local function M()
    local x = 100
    local g = Make()
    return g()
end
print(M())
print(x)
