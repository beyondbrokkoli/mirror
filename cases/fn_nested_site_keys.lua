-- EXPECT: 14
-- EXPECT: 14
-- EXPECT: 14
-- EXPECT: 6
local t = {}
t["k0"] = 7
local function Get(s) return t[s] end
local function Twice(s) return Get(s) + Get(s) end
print(Twice("k0"))
local keys = { [0] = "k0", [1] = "k0" }
local function Sel(i) return t[keys[i]] end
local function Wrap(i) return Sel(i) * 2 end
print(Wrap(0))
print(Wrap(1))
local function Add1(x) return x + 1 end
local function Drive(f, n) return f(n) end
local function Outer(n) return Drive(Add1, n) end
print(Outer(5))
