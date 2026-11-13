-- EXPECT: 4	20	0
-- EXPECT: 9	5
-- Mixed name and cell targets on one line (T2; the archived P8
-- rejection lifts with the cell-only law). The first line is the Lua
-- manual's own example (§2.4.3): 'i, a[i] = i + 1, 20' sets a[3] and
-- leaves a[4] alone, because the i in a[i] evaluates before i is
-- assigned 4. The cell store lands first (stores run right-to-left)
-- and i rebinds after it, so the key rides inline — no latch. The
-- second line swaps a cell with a name through the value temps. The
-- judge prints 4 20 0 and 9 5.
local i = 3
local a = {0, 0, 0, 0, 0}
i, a[i] = i + 1, 20
print(i, a[3], a[4])
local o = {5}
local x = 9
o[0], x = x, o[0]
print(o[0], x)
