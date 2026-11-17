-- EXPECT: 1
-- EXPECT: 2
-- The direct-name call resolves at its call site's binding point —
-- judge_divergence_audit/fn-rebind, landed 2026-11-17: the checker's
-- root fn-face home (the rider's global foundation, phase 2) now
-- carries the top-level faces on the checker's own statement-order
-- timeline, so the first print answers 1 before g() ever ran. The
-- master snapshot's defcollection drag still writes the walk's root
-- faces early (the chartered ROOT-DRAG receipt), but resolution no
-- longer reads the dragged state — the root home answers first.
local f = function() return 1 end
local g = function() f = function() return 2 end end
print(f())
g()
print(f())
