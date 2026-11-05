-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 3
-- EXPECT: 0
-- EXPECT: 0

do
  local row = {}
  row[0] = 1
  local m = {}
  m[0] = {}
  m[1] = row
end
print(sys_alloc_count())

do
  local row = {}
  row[0] = 1
  local m = {[0] = row, [1] = {7}}
end
print(sys_alloc_count())

do
  local m = {}
  m[0] = {}
  do
    local row = {}
    row[0] = 1
    m[1] = row
  end
  print(sys_alloc_count())
end
print(sys_alloc_count())

do
  local m = {}
  m[0] = {7}
  local n = {{1, 2}}
end
print(sys_alloc_count())

