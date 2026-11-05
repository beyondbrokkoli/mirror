-- EXPECT_BUILD_FAIL: Lifetime Error: 't' is moved more than once

do
  local t = {}
  t[0] = 1
  local a, b = t, t
  print(a[0], b[0])
end
