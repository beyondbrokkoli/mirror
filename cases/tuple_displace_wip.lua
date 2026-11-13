-- EXPECT_PANIC: alm runtime error: AlmTable esize
-- WIP pin for the row-displacement gap (found in T1, older than it):
-- the first target's obj latches a[0]'s row as a ghost, then the
-- second target's store displaces that row in its cell, and the
-- first store writes through the displaced ghost — the runtime
-- fatals on the corrupted header. The judge prints 0 7 (its GC keeps
-- the old row alive); the dialect's ownership has no story for a
-- latched row whose housing is overwritten mid-tuple, and the checker
-- does not yet reject the composition at compile time — the plain
-- hand-written twin (local r = a[0]; a[0] = {0, 7}; r[1] = 9) dies
-- the same way at HEAD, so the gap predates the sugar. The row-
-- housing lane owes either a compile-time rejection (a store may not
-- displace a row a live ghost aliases) or a runtime clean-fatal with
-- a named law; until then this pin keeps the gap visible.
local a = { {1, 2}, {3, 4} }
a[0][1], a[0] = 9, {0, 7}
print(a[0][0], a[0][1])
