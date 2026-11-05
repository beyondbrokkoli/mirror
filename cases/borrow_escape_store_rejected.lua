-- EXPECT_BUILD_FAIL: Lifetime Error: a row read with a dynamic or unproven key cannot be stored — its birth site is not visible to the lattice; store a row read through a literal key instead

do
  local t = {}
  t[0] = {1}
  local m = {}
  m[0] = {}
  m[1] = t[0]
end
print(sys_alloc_count())
do
  local m = {}
  m[0] = {}
  m[1] = m[0]
end
print(sys_alloc_count())
