-- P15 inclusive latch: the hand-expanded `--for-limit=inclusive`
-- numeric for — the latch is limit + 1, the guard keeps `i < limit`,
-- every other statement is the exclusive desugar. Probe-friendly
-- names; the real expansion mints for$limit and feeds the profiler.
-- Run: lua run.lua probe probes/p15_inclusive_trips.lua
-- Verdicts: `for i = 0, 10` runs 11 trips (real Lua's count; the
-- exclusive default runs 10, law case for_limit_latched); a write to
-- the user's limit var inside the body still cannot extend the loop —
-- the +1 folds once at the latch decl, so the latched-once law
-- survives the surface. Real-Lua ground truth: 11; 6 100.

local trips = 0
do
  local i = 0
  local limit = 10 + 1
  while i < limit do
    trips = trips + 1
    i = i + 1
  end
end
print(trips)

local n = 5
local latched = 0
do
  local i = 0
  local limit = n + 1
  while i < limit do
    n = 100
    latched = latched + 1
    i = i + 1
  end
end
print(latched)
print(n)
