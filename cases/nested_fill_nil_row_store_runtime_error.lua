-- FLAGS: --index-base=1
-- EXPECT_PANIC: alm runtime error: attempt to index a nil value
-- The nested-fill null-host law (post-P20 fix): under base 1 the body's
-- row store `t[i] = { i }` refuses physical -1 at trip 0 and set_core
-- reclaims the row, so the same trip's compound store `t[i][j] = ...`
-- reads the refused slot's zero word (alm_tbl_get answers from_lane(0))
-- as its host. The lowerer's walk never proved a compound host live,
-- so the fast lane declined there and the store rides the checked face
-- — whose null arm fires the faithful null-row-store guard — instead
-- of a fast-lane header deref faulting on the null host before any
-- runtime face could speak (the pre-fix death was a bare SIGSEGV,
-- receipted in probes/p20_refused_outer_null_host_fast_store.lua).
local t = {}
for i = 0, 3 do
  t[i] = { i }
  for j = 1, 2 do
    t[i][j] = i * 10 + j
  end
end
print(#t)
print(#t[1])
print(t[1][1])
print(t[3][2])
print(sys_alloc_count())
