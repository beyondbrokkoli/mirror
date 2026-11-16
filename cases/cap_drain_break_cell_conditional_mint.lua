-- EXPECT: 2
-- The same law, realistic shape — leak_contract/p13, landed
-- 2026-11-16. The closure never materializes (no value-read of h, so
-- no env mints); the per-trip box cell is minted in the body and its
-- pop free rides the back edge. Trip 1 rounds the loop; trip 2
-- breaks before the back edge — the break edge now frees the crossed
-- cell itself (the p4a treatment), so the fresh trip-2 allocation
-- no longer strands (slot-90 leak, was one alloc).
local i = 0
while true do
  local x = 5
  if i == 0 then
    local h = function() return x end
  end
  i = i + 1
  if i == 2 then break end
end
print(i)
