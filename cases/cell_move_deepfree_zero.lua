-- EXPECT: 7
-- EXPECT: 0

do
  local m = {}
  do
    local row = {}
    row[0] = 7
    m[0] = row
  end
  print(m[0][0])
end
print(sys_alloc_count())
