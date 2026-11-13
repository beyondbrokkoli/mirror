-- EXPECT: 2	1	3	4
-- The ctor-prefix tripwire (found in T2, older than it): a table
-- constructor in a target's obj position mints its table and runs
-- its entries inside the judge's pre-store window, so it latches like
-- every other observer. tag() logs the first target's key (2), then
-- the ctor's entry (1), then the values (3, 4): luajit and lua5.4
-- print 2 1 3 4. Before the latch learned ctors the dialect ran the
-- entry at store time and printed 2 3 4 1.
function tag(log, v)
  log[0] = log[0] + 1
  log[log[0]] = v
  return v
end
local log = {0, 0, 0, 0, 0}
local t = {0, 0, 0}
t[tag(log, 2) - 1], ({tag(log, 1)})[0] = tag(log, 3), tag(log, 4)
print(log[1], log[2], log[3], log[4])
