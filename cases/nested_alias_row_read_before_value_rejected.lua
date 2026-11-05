-- EXPECT_BUILD_FAIL: Type Error: a row of 't' is read before it is ever given a value
do
local t = {}
t[0] = {}
local x = t[0]
print(x[1])
end

-- EXPECT_BUILD_FAIL: Type Error: a row of 't' is read before it is ever given a value
do
local t = {}
t[0] = {}
local x = t[0]
local y = x[0]
end
