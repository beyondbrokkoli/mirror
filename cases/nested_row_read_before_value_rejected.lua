-- EXPECT_BUILD_FAIL: Type Error: a row of 't' is read before it is ever given a value
do
local t = {}
t[0] = {}
print(t[0][0])
end

-- EXPECT_BUILD_FAIL: Type Error: a row of 't' is read before it is ever given a value
do
local inner = {}
local t = {}
t[0] = inner
print(t[0][0])
end
