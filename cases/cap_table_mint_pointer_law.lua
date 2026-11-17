-- FLAGS: --index-base=1
-- EXPECT: 2
-- EXPECT: 2
-- The p9 reversal (2026-11-18): a captured Tbl binding is boxed at
-- declaration — the cell IS the binding, the env holds THE cell, and
-- a defining-scope rebind after a creation read routes through that
-- cell, so every surface (the direct one and every value-surface
-- call) prints the rebind. The judge's upvalue answer: 2 twice.
-- Reverses the 2026-11-15 alias-by-pointer pin (leak_contract/p9):
-- the Tbl face no longer stores the mint-time pointer, so the direct
-- surface and the env surface share THE storage instead of diverging
-- on the rebind.
local t = {1}
local g = function() print(t[1]) end
local h = g
t = {2}
h()
g()
