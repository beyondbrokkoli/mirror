-- EXPECT: 7	8
-- EXPECT: 1
-- The obj twin of the key capture (T2): in 't[0], t = 5, u' the name
-- store rebinds t before the cell store lands, and the judge's t[0]
-- names the OLD table. So the obj latches — tuple$o0 takes the old
-- table by move — the 5 lands in it, and the block's exit frees it:
-- one table stays live. t reads u's own cells afterwards; the judge
-- agrees (luajit and lua5.4 print nil 7 over their 1-based u, the
-- same two cells base 0 reads as 7 8).
local t = {0, 0}
local u = {7, 8}
t[0], t = 5, u
print(t[0], t[1])
print(sys_alloc_count())
