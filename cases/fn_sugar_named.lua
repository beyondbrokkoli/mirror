-- EXPECT: 2
-- EXPECT: 42
-- EXPECT: 11
function helper(x) return x + 1 end
print(helper(1))
local function double(x) return x * 2 end
print(double(21))
function calc(x) return x + 1 end
function calc(x) return x + 10 end
print(calc(1))
