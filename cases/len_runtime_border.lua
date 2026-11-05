-- ARGS: alpha beta
-- EXPECT: 3
-- EXPECT: 70000
-- EXPECT: 200001
-- EXPECT: 2
local t = {1, 2, 3}
print(#t)
local i = 3
while i < 70000 do
  t[i] = i
  i = i + 1
end
print(#t)
local s = {}
s[200000] = 7
print(#s)
print(#arg)
return t
