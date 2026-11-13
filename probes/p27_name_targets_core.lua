-- P27 (T2): the pure name-target compositions, hand-expanded into the
-- exact shape the T2 desugar will emit (value temps on one decl,
-- stores right-to-left, everything stamped at the header line).
--
-- JUDGE RECEIPTS (2026-11-13, luajit 2.1.1794214635; lua5.4 agrees):
--   x, y = y, x            -> 2  1          (scalars)
--   p, q = q, p            -> 2  1          (tables/aliases)
--   d, d = 1, 2            -> 1             (dup names: FIRST target's
--                                           value wins — stores R->L)
--   m, n = m, n            -> 5  6          (self-assign tables)
--   s, s = v, s            -> 9  table      (dup names, table values)
print(1)
do
  local x = 1
  local y = 2
  local tt0, tt1 = y, x
  y = tt1
  x = tt0
  print(x, y)
end
do
  local p = {1}
  local q = {2}
  local tt0, tt1 = q, p
  q = tt1
  p = tt0
  print(p[0], q[0])
end
do
  local d = 0
  local tt0, tt1 = 1, 2
  d = tt1
  d = tt0
  print(d)
end
do
  local m = {5}
  local n = {6}
  local tt0, tt1 = m, n
  n = tt1
  m = tt0
  print(m[0], n[0])
end
do
  local v = {9}
  local s = {0}
  local tt0, tt1 = v, s
  s = tt1
  s = tt0
  print(s[0])
end
