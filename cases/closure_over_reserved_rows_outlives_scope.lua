-- EXPECT: 0
-- EXPECT: 20
-- EXPECT: 6
-- The count rode 5 before the p9 reversal (2026-11-18): boxing the
-- captured `t` adds one live allocation at the count print — the box
-- cell holds the parent past the scope, and its drain rides the root
-- exit, after this print. 1 box + 1 closure value + 1 parent + 3 rows.
-- Closure over reserved state (battery pin of
-- probes/p25_closure_over_reserved_rows.lua): a returned function
-- value captures a table whose rows were allocated inside a fill loop,
-- the allocating frame is gone, and the closure is called after — the
-- shape the 128fc80 closure fixes came from, until now pinned only
-- compiler-side. The reads answer the stored row cells (the env keeps
-- the parent and its rows alive past the return — no premature free),
-- and sys_alloc_count() == 5 names every live allocation: 1 closure
-- value + 1 parent + 3 rows (the empty-capture differential in the
-- probe pins the closure's own heap value at exactly one). The exe
-- twin is the teardown oracle: exit 0, nothing leaks.
local function Make()
  local t = {}
  local i = 0
  while i < 3 do
    t[i] = { i, i * 10 }
    i = i + 1
  end
  local function Get(k, j) return t[k][j] end
  return Get
end
local read = Make()
print(read(0, 0))
print(read(2, 1))
print(sys_alloc_count())
