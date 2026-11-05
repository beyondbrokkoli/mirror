-- EXPECT: 6
-- EXPECT: 10
local function SimA(x) return x + 1 end
local function SimB(x) return x * 2 end
local function Drive(f, n) return f(n) end
print(Drive(SimA, 5))
print(Drive(SimB, 5))
