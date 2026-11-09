-- FLAGS: --for-limit=inclusive
-- EXPECT: 6
-- EXPECT: 0	1	2	3	4	5
-- EXPECT: 1
-- The inclusive fill at base 0: `for i = 0, n` writes user keys 0..n
-- (n + 1 stores), the latch is n + 1, and the tight reserve is
-- (n + 1) + 0 = 6 = the logical length. The guard stays `i <
-- for$limit`, so the fill rides the fast lane unchanged.
local t = {}
local n = 5
for i = 0, n do
  t[i] = i
end
print(#t)
print(t[0], t[1], t[2], t[3], t[4], t[5])
print(sys_alloc_count())
