-- EXPECT: 3	4	1	2	1	2
-- The prefix-vs-value order tripwire (T1): every target's key
-- evaluates BEFORE the RHS values — the judge's law (Lua 5.1 manual
-- §2.4.3; probed on luajit 2.1.1794214635 and lua5.4, both printing
-- 3 4 1 2 1 2). tag() appends to log in call order, so the log reads
-- keys-first (3, 4) then values (1, 2); a values-first dialect would
-- print 1 2 3 4. The compound keys (tag(...) - 1) latch into
-- tuple$k temps; the stores land t[2] = 1, t[3] = 2 right-to-left.
function tag(log, v)
  log[0] = log[0] + 1
  log[log[0]] = v
  return v
end
local log = {0, 0, 0, 0, 0}
local t = {0, 0, 0, 0}
t[tag(log, 3) - 1], t[tag(log, 4) - 1] = tag(log, 1), tag(log, 2)
print(log[1], log[2], log[3], log[4], t[2], t[3])
