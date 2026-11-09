-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 1
-- EXPECT: 10
-- EXPECT: 3
-- Combines P17 (break-arm unhoused free) + P18 (base-1 negative-start
-- refuse reclaim) + inclusive limit: trip 0 is reclaimed by set_core,
-- trip 1 is housed in t[1], and trip 2 (the inclusive boundary trip)
-- breaks before the store and reclaims its 2-level child at the break arm.
local t = {}
local bp = 2
for i = 0, 2 do
  local child = { { i * 10 } }
  if i == bp then break end
  t[i] = child
end
print(#t)
print(t[1][1][1])
print(sys_alloc_count())
