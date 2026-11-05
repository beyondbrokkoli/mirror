-- ARGS: 5 world
-- EXPECT: world
-- EXPECT: 5
-- EXPECT: world
-- EXPECT: 5
local t = {arg[0]}
local i = 1
while i < 70000 do
  t[i] = arg[i % 2]
  i = i + 1
end
print(t[69999])
print(t[42])
local s = {}
s[200000] = arg[1]
s[0] = arg[0]
print(s[200000])
print(s[0])
return t
