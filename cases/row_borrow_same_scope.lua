-- EXPECT: 8
-- EXPECT: 0

do
  local v = {{7, 8}}
  local z = v[0]
  print(z[1])
end
print(sys_alloc_count())
