-- EXPECT_BUILD_FAIL: Lifetime Error: 'b' may be moved here

do
  local a = {}
  a[0] = {}
  local b = {}
  b[0] = {}
  a[1] = b
  b[1] = a
end
print(sys_alloc_count())
