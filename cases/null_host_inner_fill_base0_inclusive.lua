-- FLAGS: --for-limit=inclusive
-- EXPECT: 4
-- EXPECT: 4
-- EXPECT: 11
-- EXPECT: 32
-- EXPECT: 5
-- The P20 shape, base-0 inclusive quadrant: the latch bound is 4, so
-- four rows house at 0..3, each ctor-reserved to 4, and t[3][2] == 32
-- (i * 10 + j at i = 3, j = 2) proves the inner fill wrote the far
-- cell of the far row. sys_alloc_count() == 5 (1 parent + 4 rows).
-- With the exclusive twin this pins both values of the for-limit
-- surface for the clean face of the shape.
local t = {}
for i = 0, 3 do
  t[i] = { i }
  for j = 1, 2 do
    t[i][j] = i * 10 + j
  end
end
print(#t)
print(#t[1])
print(t[1][1])
print(t[3][2])
print(sys_alloc_count())
