-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 2
-- EXPECT: 11
-- EXPECT: 12
-- EXPECT: 5
-- Multi-trip negative physical keys (-2, -1) storing 2-level nested
-- table constructors: set_core's refuse-reclaim must recursively free
-- both the refused outer row and its housed grandchild on trips -1 and 0.
local t = {}
for i = -1, 2 do
  t[i] = { { i + 10 } }
end
print(#t)
print(t[1][1][1])
print(t[2][1][1])
print(sys_alloc_count())
