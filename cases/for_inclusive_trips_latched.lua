-- FLAGS: --for-limit=inclusive
-- EXPECT: 3
-- EXPECT: 1
-- EXPECT: 11
-- The inclusive twin of for_limit_latched: byte-identical program,
-- one flag overturned. The first two counts are surface-independent
-- (the break cuts both loops at i == 2; the write to n latches in
-- both), and the last count is the surface itself: `for i = 0, 10`
-- runs 11 trips under inclusive, 10 under the exclusive default.
local n = 5
local trips = 0
for i = 0, n do
  n = 1
  trips = trips + 1
  if i == 2 then break end
end
print(trips)
print(n)
local c = 0
for i = 0, 10 do c = c + 1 end
print(c)
