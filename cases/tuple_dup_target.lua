-- EXPECT: 2
-- WIP pin for the Lua 5.1/LuaJIT conformance lane (T1). Duplicate
-- cell targets store left-to-right today, so the last store wins.
-- The judge disagrees: LuaJIT 2.1 and PUC Lua both print 1 (their
-- stores run right-to-left). T1 flips this pin to 1 by reversing the
-- desugar's store order; the flip is observable ONLY through
-- duplicate targets, so this case is its tripwire.
local o = {0}
o[0], o[0] = 1, 2
print(o[0])
