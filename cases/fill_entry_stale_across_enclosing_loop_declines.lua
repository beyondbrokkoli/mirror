-- EXPECT: 4
-- EXPECT: 1
-- The F1 law, probed 2026-11-11: a plan's entry facts describe the
-- loop's first entry only — the walk minted entry_lb before the
-- enclosing body's tail was recorded (`i = -2` after the inner loop),
-- and no code path ever revisited the plan. An assignment to the
-- guard after a completed loop, while a runtime loop that encloses
-- both is still active, changes the loop's re-entries (the shared
-- back edge re-runs the enclosing body), so every claim of that loop
-- dies and the stores ride the checked face: the second outer trip
-- re-enters with i = -2, the generic arm refuses the negative
-- physical keys, #t stays 4, the pre-header reserve stays the one
-- allocation, and nothing corrupts. Probed at the fix: the unsound
-- build aborted (exit 134, free(): invalid size) on this exact
-- program.
local t = {}
local i = 0
local k = 0
while k < 2 do
  while i < 4 do
    t[i] = i
    i = i + 1
  end
  i = -2
  k = k + 1
end
print(#t)
print(sys_alloc_count())
