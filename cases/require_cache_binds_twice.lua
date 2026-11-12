-- memoize by resolved path: a module required twice expands once and
-- binds twice — both names see the same module (state written
-- through one reads through the other), top-level runs exactly once.
-- FLAGS: --module-root=cases/modules
-- EXPECT: shared: expanding once
-- EXPECT: 7
-- EXPECT: 7
-- EXPECT: 9
-- EXPECT: 9
local A = require("shared.counter")
local B = require("shared.counter")
print(A.add(7))
print(B.get())
print(B.add(2))
print(A.get())
