-- EXPECT: 1
-- EXPECT: 9
-- EXPECT: 1
-- Duplicate name targets (T2): the stores land right-to-left, so the
-- FIRST target's rebind is the last to land and wins — the judge's
-- order (luajit and lua5.4 print 1), the name twin of
-- tuple_dup_target. The archived recipe's "last wins" predates T1's
-- store-order flip; the judge wins. The table lines rebind s to its
-- own old table first, then to v's — the displaced table frees on the
-- second rebind, one stays live.
local d = 0
d, d = 1, 2
print(d)
local s = {0}
local v = {9}
s, s = v, s
print(s[0])
print(sys_alloc_count())
