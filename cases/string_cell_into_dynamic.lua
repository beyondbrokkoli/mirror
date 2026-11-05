-- EXPECT: hello
local t = {1}
t[2] = "s"
local u = {"hello"}
t[0] = u[0]
print(t[0])
