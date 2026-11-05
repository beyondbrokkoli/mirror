-- EXPECT_BUILD_FAIL: '*' requires numeric operands
local op = arg[0]
if op == "mul" then
    arg[3] = (arg[1] * arg[2]) // 1024
end
return arg
