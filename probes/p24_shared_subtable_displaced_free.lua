-- P24 the aliased sub-table in the displaced-row reclaim
-- (reclaim_displaced_child → free_tbl(old_row, &[])). The reclaim frees
-- the displaced outer row with an EMPTY keeps slice, and free_tbl's
-- contains_tables walk frees every valid child pointer it finds — no
-- ownership check, no alias check. If a row being stored can legally
-- share a child with the row it displaces (both wrappers point at one
-- `sub`), freeing the displaced wrapper walks straight into `sub` and
-- frees it out from under the new row AND the live local: the read
-- through t[1][0] is a use-after-free, and the exit walks (the new
-- row's child walk, the local's own exit free) hit free_tbl's
-- FREE_POISON_ESIZE header guard — validate_header_before_free aborts
-- loudly rather than double-freeing.
--
-- Whether the share is reachable is the checker's call, and it is
-- NOT: the ctor entry MOVES `sub`, and the second `t[1] = { sub }`
-- is rejected at compile time. Measured receipt (default quadrant):
--   compile: FAILED — line 31: Lifetime Error: 'sub' may be moved
--   here — reads through a possibly-moved name are rejected at
--   compile time
-- So the runtime face can never see an aliased child through a local
-- name: reclaim_displaced_child's empty-keeps free_tbl is safe by
-- construction for this shape, and the poisoned-header guard
-- (validate_header_before_free) remains the alarm for any future
-- lowering that mints a second owner. The read-side twin (storing a
-- row read through the same sub) is the borrow_escape_store_rejected
-- family — also excluded at the mint.
-- Run: lua run.lua probe probes/p24_shared_subtable_displaced_free.lua
local sub = { 42 }
local t = {}
t[1] = { sub }
t[1] = { sub }
print(t[1][0][0])
print(sub[0])
print(sys_alloc_count())
