-- EXPECT: 140
-- EXPECT: 140
-- Closure Finding B, the definer-frame tier: F is called where the
-- call site IS the definer frame (top level), so the dynamic read of
-- x lands on the root register — the exact storage the judge's
-- upvalue rides. The write lands there too, which is why both lines
-- print 140. This is the second accidental-match tier: value-correct
-- for the wrong reason (no capture exists; the storages merely
-- coincide). Held green so the lexical fix cannot regress it.
-- LuaJIT: 140, 140
local x = 2
local function F() x = x + 40 return x end
local f = F
x = 100
print(f())
print(x)
