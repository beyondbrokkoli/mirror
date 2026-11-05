-- EXPECT: 3
-- EXPECT: hello
-- EXPECT: 2
print(1 + 2)
print("hello")
local t = {1, 2, 3}
print(t[1])
return t
