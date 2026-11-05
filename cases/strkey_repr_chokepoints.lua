-- ARGS: 1
-- EXPECT: 3
-- EXPECT: lit
-- EXPECT: ada
-- EXPECT: ada
-- EXPECT: true

local t = { name = "ada", tag = "lvl" }
local s = "x"
local i = 0
while i < 3 do
    s = "lit"
    i = i + 1
end
print(#s)
print(s)
local d = "name"
print(t[d])
local m = t.missing
t[m] = "zz"
print(t.name)
t.tag = m
print(#t.tag == 0)
return arg
