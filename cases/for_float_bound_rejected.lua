-- EXPECT_BUILD_FAIL: Syntax Error: for-loop bounds must be integers
for i = 0, 2.5 do
  print(i)
end
