-- EXPECT: 20
-- EXPECT: 7
-- EXPECT: 2
-- EXPECT: 9
-- EXPECT: 2
-- EXPECT: 9
-- EXPECT: 40
-- EXPECT: 9
-- EXPECT: 0
-- EXPECT: 5
-- EXPECT: 0
-- P23 promoted (probes/p23_guard_offset_cross_iteration_alias.lua):
-- the fast-lane claim law now books per (loop, base) group, not per
-- (loop, base, shape). Two distinct key shapes over one guard —
-- `t[i]` and `t[i+1]` — overlap across trips (at trip i, slot i then
-- slot i+1; at trip i+1, slot i+1 again), so the whole group collapses
-- to the checked face and widens nothing: the forward arm and the
-- backward-order arm (offset store first, guard store second — the
-- collapse does not care which statement wins the sighting) both keep
-- their 4-pair ledgers owned by alm_tbl_set_row's replace-reclaim.
-- Control a: the P21 law — same shape twice per trip, the first
-- sighting's slots fresh, the repeat riding the checked face (a + 3
-- pairs). Arm s: the row-stored law — loop 2's row fill rides the
-- checked face because loop 1's rows are walk-lefthind memory (s + 5
-- pairs — the sequential twin of the group law; scalar-only fills
-- keep the lane via the cells-hold-one-kind law, see
-- runtime_far_store_between_fills). Arm n: the depth law — a nested
-- fill's stores re-run once per enclosing trip, so the depth counter
-- takes them to the checked face (n + 2 pairs; loop 1 exited with rows
-- behind it, and the j-replace reclaims its own). 7 = a + 3 pairs,
-- 9 = 4 pairs + host, 5 = n + 2 pairs; every block drains to 0.
-- Pre-fix ledgers: the two-shape arms lost 2 pairs each (exit 4, the
-- slot-90 tell), the nested fill leaked in every variant (exit 4),
-- the sequential twin leaked at exit 6 (4 Safe claims = 6).
do
  local a = {}
  for i = 0, 3 do
    a[i] = { { i } }
    a[i] = { { i * 10 } }
  end
  print(a[2][0][0])
  print(sys_alloc_count())
end
do
  local t = {}
  for i = 0, 3 do
    t[i] = { { i } }
    t[i+1] = { { i * 10 } }
  end
  print(t[2][0][0])
  print(sys_alloc_count())
end
do
  local b = {}
  for i = 0, 3 do
    b[i+1] = { { i * 10 } }
    b[i] = { { i } }
  end
  print(b[2][0][0])
  print(sys_alloc_count())
end
do
  local s = {}
  for i = 0, 2 do
    s[i] = { { i } }
  end
  for k = 3, 5 do
    s[k] = { { k * 10 } }
  end
  print(s[4][0][0])
  print(sys_alloc_count())
end
do
  local n = {}
  local i = 0
  while i < 2 do
    local j = 0
    while j < 2 do
      n[i] = { { j } }
      j = j + 1
    end
    i = i + 1
  end
  print(n[1][0][1])
  print(sys_alloc_count())
end
print(sys_alloc_count())
