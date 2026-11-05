-- EXPECT_BUILD_FAIL: Lifetime Error: 'x' may be moved here
-- EXPECT_BUILD_FAIL: Lifetime Error: 'x' may be moved here

do
  local x = {}
  x[0] = 5
  local y = x
  print(x[0])
end
do
  local x = {}
  x[0] = 5
  local y = x
  x[1] = 7
end
