-- EXPECT: hello
-- EXPECT: 5
-- EXPECT: 2.5
-- EXPECT: true
-- EXPECT: 5
-- EXPECT: grown
-- EXPECT: 5
-- EXPECT: 5
local t = {"hello", 5, 2.5, true}
print(t[0])
print(t[1])
print(t[2])
print(t[3])
print(#t[0])
t[4] = "grown"
print(t[4])
print(#t)
print(#t[4])
return t
