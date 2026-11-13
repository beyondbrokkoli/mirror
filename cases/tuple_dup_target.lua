-- EXPECT: 1
-- The store-order tripwire (T1): duplicate cell targets store
-- right-to-left — the judge's order — so the FIRST target's store is
-- the last to land and wins. LuaJIT 2.1 and PUC Lua both print 1;
-- the dialect matches since T1 flipped the desugar's store order.
-- Observable ONLY through duplicate targets, which is why this case
-- exists.
local o = {0}
o[0], o[0] = 1, 2
print(o[0])
