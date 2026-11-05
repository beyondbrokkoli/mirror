-- EXPECT_BUILD_FAIL: Type Error: divergent expansions cannot meet at one call
local function SimA(x) return x + 1 end
local function SimB(x) return x * 2 end
local function Choose(c, a, b)
    if c then
        return a
    else
        return b
    end
end
local function Drive(f, n) return f(n) end
print(Drive(Choose(true, SimA, SimB), 5))
