-- EXPECT: 3	9
-- EXPECT: 3
-- EXPECT: 5
-- EXPECT: 4	7
-- The scalar-node law: a scalar node rides its name's binding depth.
-- A nested assignment must not orphan the node in an inner map that
-- dies with its scope — before the fix, a call-result temp assigned to
-- an OUTER name from inside a Do parked the node in the Do's map, so
-- the name's type degraded to Pending when the scope closed and any
-- later typed rebind false-rejected ('type conflict — Pending vs
-- Integer'). The tuple sugar manufactures exactly this shape (value
-- temps on a Do decl, then name stores), so idiomatic
-- z, x = id(7), 9 followed by z = 3 was rejected while the plain twin
-- passed. The judge (luajit, lua5.4) accepts all three programs.
local function id(v)
  return v
end

-- The sugar shape: name-target tuple, then a later typed rebind.
local z = 3
local x = 0
z, x = id(7), 9
z = 3
print(z, x)

-- The hand-written twin: same composition, no sugar.
local w = 3
do
  local t0 = id(7)
  w = t0
end
w = 3
print(w)

-- The shadow direction: an inner shadow must take its own depth and
-- leave the outer name's node alone.
local n = id(1)
do
  local n = 3
  n = id(5)
  print(n)
end
n = 4
print(n, id(7))
