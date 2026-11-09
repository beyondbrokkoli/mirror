-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 2
-- Zero-trip pre-header reserve boundaries under base-1 inclusive with
-- row housing (P14 + P16): `for i = 1, 0` cancels to (0 + 1) + (-1) = 0,
-- while `for i = 2, 1` sizes (1 + 1) + (-1) = 1 unwritten contains_tables
-- slot that free_tbl safely skips via is_valid_ptr at exit.
local a = {}
for i = 1, 0 do
  a[i] = { i }
end
local b = {}
for i = 2, 1 do
  b[i] = { i }
end
print(#a)
print(#b)
print(sys_alloc_count())
