-- P12 drop planning over a reserved site: mint a parent, fill it with
-- fresh rows under a conversion loop (pre-header reserve on the parent,
-- ctor reserve borrowing the loop bound for each row), borrow one row
-- out, unhook the parent name, drop the row.
-- Run: lua run.lua probe probes/p12_drop_over_reserved_site.lua
-- Verdicts: prints 1, 1, then 0; the .ll carries alm_tbl_reserve before
-- the loop and at each row store; the parent nil-drop lowers
-- alm_tbl_free_except keeping the borrowed row's register, the row
-- nil-drop lowers alm_tbl_free, in that order; the runtime sidecar
-- shows RT_ALLOC x4, RT_FREE + RT_FREE_CHILD x2 (the two unkept rows)
-- at the parent drop, RT_FREE (the kept row) at the row drop;
-- sys_alloc_count() == 0 at exit.

local outer = {}
for i = 0, 3 do
  outer[i] = {i}
end
local row = outer[1]
print(row[0])
outer = nil
print(row[0])
row = nil
print(sys_alloc_count())
