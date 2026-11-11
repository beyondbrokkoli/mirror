-- EXPECT: 4
-- EXPECT: 1
-- The F3 law, probed 2026-11-11 (the worklist's unprobed third
-- variant): a function parameter used to inherit an outer same-named
-- literal's recorded shapes — walk_fn_def never entered params into
-- the numeric environment, so a loop over the parameter read the
-- outer `local i = 0`'s [Lit(0)], minted a plan with entry_lb = 0,
-- and the lane admitted fill(-2) into a raw getelementptr at
-- data[-2] (exit 134, free(): invalid size on the unsound build).
-- Parameters now enter as the unknowable shape: the nonneg gate
-- declines the plan outright, the fill rides the generic growth
-- faces, the negative physical keys refuse, and the constructor
-- stays the one allocation.
local t = {}
local i = 0
local function fill(i)
  while i < 4 do
    t[i] = i
    i = i + 1
  end
end
fill(-2)
print(#t)
print(sys_alloc_count())
