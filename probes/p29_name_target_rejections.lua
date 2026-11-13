-- P29 (T2): the pinned-arg rejection through the name-target value
-- latch — the P3 receipt carried into T2's shape. `x, y = arg, x`
-- desugars to `local tt0 = arg` on the temp decl: a direct move
-- of the boundary table, which the pin law refuses. The rejection
-- must land at the TUPLE HEADER's line (attribution law), not at the
-- decl's — the whole expansion carries the header's stamp.
--
-- EXPECTED: compile fails — "the boundary table 'arg' is pinned".
-- (Judge receipt: legal Lua, but the dialect's pin law keys on the
-- boundary header; this is a named rejection, not a judge match.)
print(1)
local x = {0}
local y = {1}
local tt0, tt1 = arg, x
y = tt1
x = tt0
print(x[0], y[0])
