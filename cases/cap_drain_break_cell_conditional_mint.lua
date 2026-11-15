-- EXPECT: 2
-- XFAIL: leak_contract/p13 — same law, realistic shape
-- The creation read runs on trip 1 only: its env mints and escapes
-- with the trip-1 cell. Trip 2 mints a fresh unheld cell and breaks
-- before any creation read — the boundary drain frees the envs it
-- knows, but the trip-2 cell was inventory-consumed by the pop on
-- the normal edge, so the break edge strands it (slot-90 leak).
-- Value law correct (EXPECT pins the run); flip the pin when the
-- break edge carries the crossed-cell frees.
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
