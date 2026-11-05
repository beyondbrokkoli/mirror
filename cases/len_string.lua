-- EXPECT: 5
-- EXPECT: 8
-- EXPECT: 3
-- EXPECT: 8
-- EXPECT: 0
local s = "hello"
print(#s)
print(#"literal!")
local t = {1, 2, 3}
print(#t)
print(#s + #t)
print(#"")
