-- FLAGS: --index-base=1
-- EXPECT: 1
-- EXPECT: 0
-- The base-1 constructor spelling of the refused store: the literal
-- [0] shifts to physical -1 at the parse mint, the ctor's checked
-- table_set no-ops, and the refuse path reclaims the row {1} the
-- static pass had already housed -- the table alone survives, the
-- border stays 0, and sys_alloc_count() exits clean. Completes
-- ctor_zero_key_noop's scalar law with the owned row under the same
-- key.
local u = { [0] = {1} }
print(sys_alloc_count())
print(#u)
