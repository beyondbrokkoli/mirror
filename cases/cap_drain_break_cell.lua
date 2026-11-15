-- EXPECT: 5
-- XFAIL: leak_contract/p13 — break edges carry no crossed-cell frees yet
-- The break edge over a declare-minted captured cell: the body pop's
-- alm_tbl_free rides the normal back edge only, and the pop consumes
-- the binding_cells inventory, so neither the break edge nor the
-- boundary drain can free the cell (slot-90 leak, one alloc). The
-- value law is already correct — EXPECT pins the clean run. When the
-- break edge gets the p4a treatment (crossed-cell frees emitted on
-- the edge itself), the envelope empties: flip the pin (drop the
-- xfail line) and relock.
print(5)
while true do
  local x = 5
  if x == 5 then break end
  local h = function() return x end
end
