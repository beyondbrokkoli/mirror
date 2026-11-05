-- EXPECT_BUILD_FAIL: Syntax Error: for-step is not in the dialect — 'for i = a, b do' has start and limit only
for i = 0, 10, 2 do
  print(i)
end
