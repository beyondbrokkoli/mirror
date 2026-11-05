
-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 7
-- EXPECT: 0
do
  local t = {1}
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t[0] = 7
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 3
-- EXPECT: 0
do
  local t = {}
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t[0] = 3
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 2.5
-- EXPECT: 0
do
  local t = {}
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t[0] = 2.5
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 5
-- EXPECT: 0
do
  local t = {}
  t[0] = 1
  local u = t
  t = {}
  print(sys_alloc_count())
  print(u[0])
  t[0] = 5
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 1
-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 0
do
  local t = {1}
  print(sys_alloc_count())
  t = nil
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t[0] = 2
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 41
-- EXPECT: 0
do
  local x
  x = {}
  x[0] = 41
  print(x[0])
end
print(sys_alloc_count())

-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 7
-- EXPECT: 0
do
  local t2 = {}
  local t = {1}
  print(sys_alloc_count())
  t = t2
  print(sys_alloc_count())
  t[0] = 7
  print(t[0])
end
print(sys_alloc_count())

-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 1.5
-- EXPECT: 0
do
  local t = {}
  t[0] = 0.5
  print(sys_alloc_count())
  t = {}
  print(sys_alloc_count())
  t[0] = 1.5
  print(t[0])
end
print(sys_alloc_count())
