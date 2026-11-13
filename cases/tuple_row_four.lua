-- EXPECT: 38	383	728	1073
-- EXPECT: 5	0
local a00, a10, a20, a30 = 3, 1, 4, 1
local b00, b01, b02, b03 = 5, 9, 2, 6
local o = {0, 0, 0, 0}
o[0], o[1], o[2], o[3] = a00*b00+a10*b01+a20*b02+a30*b03,
                         a00*b00+a10*b01+a20*b02+a30*b03+345,
                         a00*b00+a10*b01+a20*b02+a30*b03+690,
                         a00*b00+a10*b01+a20*b02+a30*b03+1035
print(o[0], o[1], o[2], o[3])
-- The dependent-key tail (T1): every target's table and key evaluates
-- before any store (Lua 5.1 manual §2.4.3), so the second target's key
-- t[0] latches as 0 — the OLD value — and the store lands in t[0],
-- not t[5]. LuaJIT 2.1 and PUC Lua both print 5 0; the dialect matches
-- since T1 latched the target prefixes with the RHS temps.
local t = {0, 0, 0, 0, 0, 0, 0}
t[0], t[t[0]] = 5, 6
print(t[0], t[5])
