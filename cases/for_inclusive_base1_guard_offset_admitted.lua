-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 4
-- EXPECT: 0
-- EXPECT: 30
-- EXPECT: 5
-- Guard-offset admission boundary under base-1 inclusive: i starts at 0,
-- but key (i + 1) shifts k_phys to (+1 - 1) = 0, satisfying
-- first + k_phys >= 0. All 4 trips ride the fast lane with K_eff = 0.
local t = {}
for i = 0, 3 do
  t[i + 1] = { i * 10 }
end
print(#t)
print(t[1][1])
print(t[4][1])
print(sys_alloc_count())
