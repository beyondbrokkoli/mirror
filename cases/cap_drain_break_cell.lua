-- EXPECT: 5
-- The break edge over a declare-minted captured cell — leak_con
-- tract/p13, landed 2026-11-16: the body pop's cell free used to
-- ride the back edge only, so the breaking trip's box cell stranded
-- (slot-90 leak, one alloc). The break edge now carries the
-- crossed-cell frees (the p4a treatment, emit_break_edge_cell_frees),
-- path-exclusive with the pop's back-edge free: each execution frees
-- exactly once. No env mints in a loop body — V1 keeps creation
-- reads straight-line and closures materialize at their first
-- value-read — so the crossed cells are unheld at the edge.
print(5)
while true do
  local x = 5
  if x == 5 then break end
  local h = function() return x end
end
