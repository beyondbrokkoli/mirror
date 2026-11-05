-- EXPECT_BUILD_FAIL: Type Error: '+' requires numeric operands
if arg[0] then
    print(1)
end
print(arg[0] + 1)
