-- EXPECT: 42
-- EXPECT: 42
-- Closure Finding B, the accidental-match tier: the top-level fn
-- literal captures nothing (walk_fn_def's skip(1) starts the chain
-- past depth 0), so Get's body resolves x DYNAMICALLY at the call
-- site — and with no shadow at any intervening frame, the dynamic
-- read falls through to the definer's root binding and matches the
-- judge by construction, not by capture. This pin holds the
-- fall-through tier so the eventual lexical fix (drop the skip +
-- box-is-binding) cannot regress it: the same program must keep
-- printing 42, 42 whether the name is resolved dynamically (today)
-- or through a box cell (the fix).
-- LuaJIT: 42, 42
local x = 2
local function Make()
    local function Get()
        x = x + 40
        return x
    end
    return Get
end
local g = Make()
print(g())
print(x)
