-- EXPECT_BUILD_FAIL: Lifetime Error: 'c' may be moved here
local c = {}
c[0] = 7
do
  local b = c
end
print(sys_alloc_count())
print(c[0])
