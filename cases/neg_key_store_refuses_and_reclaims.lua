-- EXPECT: 1
-- EXPECT: 0
-- A store the destination refuses must reclaim its row: the static
-- pass already moved {1} into t's housing, so when the runtime's
-- negative-index no-op drops the write, the refuse path frees the
-- incoming child instead of leaking it (the header's
-- contains_tables bit is the same signal free_tbl trusts). Only t
-- survives at the count, and the border never grows. The landed
-- twin of the base-1 constructor spelling
-- (index_base1_ctor_zero_key_row_reclaimed).
local k = -1
local t = {}
t[k] = {1}
print(sys_alloc_count())
print(#t)
