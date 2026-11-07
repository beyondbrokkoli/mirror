-- FLAGS: --index-base=1
-- EXPECT: 8
-- EXPECT: 1	8
-- A fill guarded by an if still rides the fast lane: the lane scans
-- into both arms with cloned frames, so the conditional t[i] = i is
-- claimed while the guard is live, widen folds the bare guard's -1,
-- and the reserve tightens to 8 exactly like the unguarded golden
-- fill -- an if arm is not a decline (only a dead guard, a missed
-- key shape, or a failed admission gate declines).
local t = {}
for i = 1, 9 do
  if i > 0 then
    t[i] = i
  end
end
print(#t)
print(t[1], t[8])
