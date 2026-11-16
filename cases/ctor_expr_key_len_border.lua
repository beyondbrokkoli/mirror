-- EXPECT: 1
-- EXPECT: 2
-- XFAIL: judge_divergence_audit/reserve-border — line 1 only: the
-- explicit key 1 raises the reserve to 2; the judge's border is 1.
-- Line 2 stays dialect-pinned: under the default base the positional
-- 1 and [k] land on distinct keys 0 and 1, so its border is the
-- judge's answer shifted by the base — the declared surface, not
-- this pin. Flip with the # border law.
do
  print(#{[0 + 1] = 7})
end
do
  local k = 1
  local t = {1, [k] = 2}
  print(#t)
end
