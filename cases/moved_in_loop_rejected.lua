-- EXPECT_BUILD_FAIL: Lifetime Error: 'w' may be moved here

do
  local w = {3, 4}
  local x = {1, 2}
  local i = 0
  while i < 2 do
    x = w
    i = i + 1
  end
end
print(sys_alloc_count())
