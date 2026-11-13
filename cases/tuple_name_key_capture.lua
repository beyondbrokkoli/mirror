-- EXPECT: 4	20	0
-- The key-capture tripwire (T2): 'b[j], j = 20, j + 1' rebinds j
-- BEFORE the cell store lands — stores run right-to-left — yet the
-- judge's b[j] reads the OLD j: every target's key evaluates before
-- any store (Lua 5.1 manual §2.4.3; luajit and lua5.4 print 4 20 0).
-- So a cell prefix latches the names that name targets to its RIGHT
-- rebind, and the key rides tuple$k0. An inline key would read the
-- new j and print 4 0 20. The mirror shape (tuple_mixed_targets,
-- 'i, a[i] = i + 1, 20') needs no latch: its name rebinds after.
local j = 3
local b = {0, 0, 0, 0, 0}
b[j], j = 20, j + 1
print(j, b[3], b[4])
