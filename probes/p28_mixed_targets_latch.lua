-- P28 (T2): mixed name/cell targets — the latch-rule conundrums,
-- hand-expanded into the T2 desugar shape.
--
-- JUDGE RECEIPTS (2026-11-13, luajit 2.1.1794214635; lua5.4 agrees):
--   i, a[i] = i+1, 20      -> 4  20  0   (cell key after the name in
--                                          target order: name store
--                                          lands FIRST (R->L) but the
--                                          cell store still lands
--                                          before i rebinds? NO —
--                                          target 1 (cell) stores
--                                          first, i rebinds after; a
--                                          plain inline key reads old
--                                          i either way IF the cell
--                                          target has the HIGHER
--                                          index)
--   b[j], j = 20, j+1      -> 4  20  0   (KEY CAPTURE: the name store
--                                          (target 1) lands before the
--                                          cell store (target 0), so
--                                          the key MUST latch or the
--                                          store writes b[4])
--   t[0], t = 5, u         -> nil 7      (OBJ DISPLACEMENT: the store
--                                          goes into the OLD t; obj
--                                          must latch — a move)
--
-- The judge's line (Lua 5.1 manual §2.4.3, probed): every target's
-- table and key evaluates before any store, then stores land
-- right-to-left. Names have no prefix; the latch rule for names keys
-- on "a name-target store lands before this cell store" — i.e. the
-- name target's index is HIGHER than the cell target's.
print(1)
do
  -- i, a[i] = i+1, 20 : cell target index 1 > name index 0; the name
  -- store lands AFTER the cell store, so the inline key reads old i.
  local a = {0, 0, 0, 0}
  local i = 3
  local tt0, tt1 = i + 1, 20
  a[i] = tt1
  i = tt0
  print(i, a[3], a[4])
end
do
  -- b[j], j = 20, j+1 : name index 1 > cell index 0; the name store
  -- lands FIRST, so the key latches into tk0.
  local b = {0, 0, 0, 0}
  local j = 3
  local tk0, tt0, tt1 = j, 20, j + 1
  j = tt1
  b[tk0] = tt0
  print(j, b[3], b[4])
end
do
  -- t[0], t = 5, u : the obj is itself a name target storing first;
  -- it latches (a move), the store rides the latch.
  local u = {7, 8}
  local t = {0, 0}
  local to0, tt0, tt1 = t, 5, u
  t = tt1
  to0[0] = tt0
  print(t[0], t[1])
end
