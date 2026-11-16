-- EXPECT: 13	0	4	8
-- XFAIL: judge_divergence_audit/reserve-panic — the pre-header's
-- checked bound + K_eff sum overflows i64 and aborts before the
-- first store; the judge allocates lazily and answers 13 0 4 8.
-- Flip with the reserve-panic law.
-- A fill loop whose bound + K_eff overflows i64. The pre-header
-- widening is the runtime's checked sum, so the loud fatal fires
-- before the first fast store — a silent skip would leave the span
-- short and the unchecked stores writing past it (this shape
-- segfaulted before the checked seam existed). The return-exit keeps
-- the plan alive: has_break declines, return does not.
local function fill()
  local t = {}
  local i = 0
  while i < 9223372036854775807 do
    t[i + 5] = i
    i = i + 1
    if i > 8 then return t end
  end
end
local t = fill()
print(#t, t[5], t[9], t[13])
