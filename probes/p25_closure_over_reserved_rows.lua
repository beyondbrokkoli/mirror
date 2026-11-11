-- P25 closure over reserved state (W2 item 3, drafted 2026-11-12): a
-- returned function value captures a table whose rows were allocated
-- inside a fill loop, the allocating frame returns and is gone, and
-- the closure is called after. The macro/expansion system is pinned
-- compiler-side (17 h12 tests) but nothing in the corpus executes this
-- shape — the class the 128fc80 closure fixes came from. The ledger is
-- the oracle: the rows must survive the frame exit (no premature free)
-- and none may leak at exit (the atexit walk stays quiet).
-- (First draft used `local Get` + late assign + name call: the checker
-- rejects that — "call it through its value". The Make/returned-closure
-- spelling is the corpus idiom, escape_env_shared_siblings.)
-- Run: lua run.lua probe probes/p25_closure_over_reserved_rows.lua
-- Verdicts: read answers the stored row cells (the env keeps the
-- parent and its rows alive past the return); sys_alloc_count() == 5
-- at print (1 closure value + 1 parent + 3 rows — the empty-capture
-- differential pins the closure's own heap value at exactly one
-- alloc); the process exits 0 with no allocator tell.
-- Face note: `alm -o x.so` writes ET_DYN with entry 0 — executing the
-- .so directly segfaults by construction; the faces are hosted and
-- --exe, and both are clean here (the exe twin is the teardown
-- oracle).

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
