
-- EXPECT: === alm soundness gauntlet ===
-- EXPECT: -- S1 integer law sheet --
-- EXPECT: -9223372036854775808
-- EXPECT: -9223372036854775808
-- EXPECT: 9223372036854775807
-- EXPECT: -9223372036709301616
-- EXPECT: 3.5	-3.5	-3.5	3.5
-- EXPECT: 3	-4	-4	3
-- EXPECT: 1	1	-1	-1
-- EXPECT: true	true	false	true	true	true
-- EXPECT: true	true	false	true
-- EXPECT: -- S2 float law sheet --
-- EXPECT: 3.5	-4	-4	0.5
-- EXPECT: 0.30000000000000004
-- EXPECT: inf	-inf	NaN
-- EXPECT: -- S3 control flow and phis --
-- EXPECT: 30
-- EXPECT: true
-- EXPECT: 1
-- EXPECT: 3	7
-- EXPECT: 30	60
-- EXPECT: mid
-- EXPECT: 3
-- EXPECT: -- S4 shadowing and multi-name locals --
-- EXPECT: 100
-- EXPECT: 1
-- EXPECT: 7	seven
-- EXPECT: 9	true
-- EXPECT: -- S5 constructors, '#', identity --
-- EXPECT: 2
-- EXPECT: 2
-- EXPECT: 3
-- EXPECT: 3
-- EXPECT: 0
-- EXPECT: 1	2	3	4
-- EXPECT: 10	20	0	40
-- EXPECT: true
-- EXPECT: true
-- EXPECT: false	true
-- EXPECT: 0
-- EXPECT: -- S6 absence is a well-defined zero --
-- EXPECT: 0	0	0
-- EXPECT: 0	5
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: false
-- EXPECT: 0
-- EXPECT: -- S7 flow-insensitive reads --
-- EXPECT: 0
-- EXPECT: 5
-- EXPECT: 0
-- EXPECT: -- S8 first touch through aliases and twins --
-- EXPECT: 1.5
-- EXPECT: 5	9
-- EXPECT: 0
-- EXPECT: -- S9 strings and bool tables --
-- EXPECT: compiler	runtime
-- EXPECT: second
-- EXPECT: true	false	true	false	false
-- EXPECT: 0
-- EXPECT: -- S10 nested rows, lineage, store links --
-- EXPECT: 8
-- EXPECT: 7
-- EXPECT: 0
-- EXPECT: 1	1
-- EXPECT: 0
-- EXPECT: -- S11 sparse worlds --
-- EXPECT: 9	10	0
-- EXPECT: 0
-- EXPECT: 1	0
-- EXPECT: 0
-- EXPECT: 9	81	0	7
-- EXPECT: 0
-- EXPECT: 4242	1
-- EXPECT: 0
-- EXPECT: -- S12 the VM world (1 MiB graduation) --
-- EXPECT: 0	393213	0	42	0
-- EXPECT: 0
-- EXPECT: -- S13 deep-free ledger --
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 7	8
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: -- S14 the lifetime doctrine --
-- EXPECT: 7
-- EXPECT: 9
-- EXPECT: 0
-- EXPECT: survived double nil
-- EXPECT: 0
-- EXPECT: loop drop ok
-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: dropped eagerly - the phi is the sole carrier
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 22
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: -- S15 closing ledger: the leaks the discipline admits --
-- EXPECT: 2
-- EXPECT: 4
-- EXPECT: 0
-- EXPECT: 1.5
-- EXPECT: 2.5	1.5
-- EXPECT: 0
-- EXPECT: gauntlet complete

print("=== alm soundness gauntlet ===")

print("-- S1 integer law sheet --")
print(9223372036854775807 + 1)
local mn = -9223372036854775807 - 1
print(mn)
print(mn - 1)
print(3037000500 * 3037000500)
print(7 / 2, -7 / 2, 7 / -2, -7 / -2)
print(7 // 2, -7 // 2, 7 // -2, -7 // -2)
print(7 % 2, -7 % 2, 7 % -2, -7 % -2)
print(1 < 2, 2 <= 2, 3 > 4, 4 >= 4, 1 == 1, 1 ~= 2)
print(true and false or true, not (3 == 4), true == false, true ~= false)

print("-- S2 float law sheet --")
print(7.0 / 2.0, -7.5 // 2.0, 7.5 // -2.0, -7.5 % 2.0)
print(0.1 + 0.2)
print(1.0 / 0.0, -1.0 / 0.0, 0.0 / 0.0)

print("-- S3 control flow and phis --")
local acc = 0
local i = 1
while i <= 10 do
    if i % 2 == 0 then
        acc = acc + i
    end
    i = i + 1
end
print(acc)

local on = true
local j = 0
while j < 7 do
    if j == 2 then
        on = false
    end
    if j == 5 then
        on = true
    end
    j = j + 1
end
print(on)

local firing = 0 < 1
local k = 0
while firing do
    k = k + 1
    firing = 0 < 0
end
print(k)

local a = 0
local b = 10
while a < 4 and b > 7 do
    a = a + 1
    b = b - 1
end
print(a, b)

local s = {}
local n = 0
while n < 64 do
    s[n] = 1
    n = n + 1
end
s[30] = 60
local bound = 60
local p = 0
while s[p] < bound do
    p = p + 1
end
print(p, s[30])
s = nil

local v = 42
if v < 0 then
    print("neg")
elseif v == 0 then
    print("zero")
elseif v < 100 then
    print("mid")
else
    print("big")
end

local steps = 0
local guard = true
while guard or steps > 3 do
    steps = steps + 1
    if steps == 3 then
        guard = false
    end
end
print(steps)

print("-- S4 shadowing and multi-name locals --")
do
    local x = 1
    if 0 < 1 then
        local x = 100
        print(x)
    end
    print(x)
    local a, b = 7, "seven"
    print(a, b)
    local c, d
    c = 9
    d = true
    print(c, d)
end

print("-- S5 constructors, '#', identity --")
do
    local m = {{1, 2}, {3, 4}}
    print(#m)
    local u = m
    print(#u)
    local p3 = {1, 2, 3}
    print(#p3)
    local q3 = p3
    print(#q3)
    local e3 = {}
    print(#e3)
    print(u[0][0], u[0][1], u[1][0], u[1][1])
    local c = {10, 20, [3] = 40}
    print(c[0], c[1], c[2], c[3])
    local e = {}
    local w = e
    print(w == w)
    w[0] = 1
    print(w == w)
    local f = {}
    f[0] = 1
    print(w == f, w ~= f)
end
print(sys_alloc_count())

print("-- S6 absence is a well-defined zero --")
do
    local g = {}
    g[0] = 1
    g[1] = 2
    print(g[9], g[-1], g[1000000])
    g[9] = 5
    print(g[3], g[9])
    g[-1] = 99
    print(g[-1])
    local f = {}
    f[0] = 1.5
    print(f[5])
    local bt = {}
    bt[0] = true
    print(bt[3])
end
print(sys_alloc_count())

print("-- S7 flow-insensitive reads --")
do
    local m = {}
    print(m[0])
    m[0] = 5
    print(m[0])
end
print(sys_alloc_count())

print("-- S8 first touch through aliases and twins --")
do
    local t = {}
    local u = t
    u[0] = 1.5
    print(u[0])
    local p = {}
    local i = 0
    while i < 8 do
        p[i] = i + 2
        i = i + 1
    end
    print(p[3], p[7])
end
print(sys_alloc_count())

print("-- S9 strings and bool tables --")
do
    local names = {}
    names[0] = "compiler"
    names[1] = "runtime"
    print(names[0], names[1])
    local pick = "first"
    if 1 > 2 then
        pick = "first"
    else
        pick = "second"
    end
    print(pick)
    local flags = {}
    local i = 0
    while i < 4 do
        flags[i] = i % 2 == 0
        i = i + 1
    end
    print(flags[0], flags[1], flags[2], flags[3], flags[7])
end
print(sys_alloc_count())

print("-- S10 nested rows, lineage, store links --")
do
    local m = {{7, 8}}
    print(m[0][1])
    local x = m[0]
    print(x[0])
    print(m[5][0])
    local g = {}
    g[0] = {}
    g[0][0] = 1
    local row2 = g[0]
    print(g[0][0], row2[0])
end
print(sys_alloc_count())

print("-- S11 sparse worlds --")
do
    local q = {}
    q[100001] = 9
    q[100002] = 10
    print(q[100001], q[100002], q[7])
end
print(sys_alloc_count())
do
    local d = {}
    d[100000] = 1
    print(d[100000], d[99999])
end
print(sys_alloc_count())
do
    local s = {}
    local i = 0
    while i < 10 do
        s[i] = i * i
        i = i + 1
    end
    s[200000] = 7
    print(s[3], s[9], s[100], s[200000])
end
print(sys_alloc_count())
do
    local b = {}
    b[0] = 1
    local idx = 1
    local j = 0
    while j < 63 do
        idx = idx * 2
        j = j + 1
    end
    idx = idx - 1
    b[idx] = 4242
    print(b[idx], b[0])
end
print(sys_alloc_count())

print("-- S12 the VM world (1 MiB graduation) --")
do
    local v = {}
    local i = 0
    while i < 131072 do
        v[i] = i * 3
        i = i + 1
    end
    v[200000] = 42
    print(v[0], v[131071], v[131072], v[200000], v[300000])
end
print(sys_alloc_count())

print("-- S13 deep-free ledger --")
do
    local t = {1, 2, 3}
end
print(sys_alloc_count())
do
    local t = {{1}, {2}}
end
print(sys_alloc_count())
do
    local m = {}
    m[0] = {7, 8}
    print(m[0][0], m[0][1])
end
print(sys_alloc_count())
do
    local c = {7}
    local b = c
end
print(sys_alloc_count())
do
    local x
    if 1 < 2 then
        local w = {}
        w[0] = 7
        x = w
    end
end
print(sys_alloc_count())

print("-- S14 the lifetime doctrine --")
do
    local t = {}
    t[0] = 7
    local u = t
    t = nil
    print(u[0])
    u = nil
    t = {}
    t[1] = 9
    print(t[1])
end
print(sys_alloc_count())
do
    local t = {}
    t[0] = 41
    t = nil
    t = nil
    print("survived double nil")
end
print(sys_alloc_count())
do
    local t = {}
    t[0] = 41
    local i = 0
    while i < 3 do
        t = nil
        i = i + 1
    end
    print("loop drop ok")
end
print(sys_alloc_count())
do
    local a = {}
    a[0] = 1
    local b = {}
    b[0] = 2
    local target = a
    if false then
        target = b
    end
    target[0] = 5
    target = nil
    print(sys_alloc_count())
    print("dropped eagerly - the phi is the sole carrier")
end
print(sys_alloc_count())
do
    local a = {}
    a[0] = 1
    local target
    target = a
    target = nil
    print(sys_alloc_count())
end
print(sys_alloc_count())
local row = {11, 22}
do
    local m = {}
    m[0] = row
    print(m[0][1])
end
print(sys_alloc_count())
row = nil
print(sys_alloc_count())

print("-- S15 closing ledger: the leaks the discipline admits --")
print(#{10, 20})
do
    local pick = {}
    pick[0] = 3
    if 1 < 2 then
        local w = {}
        w[0] = 4
        pick = w
    end
    print(pick[0])
end
print(sys_alloc_count())
do
    local t = {}
    local u = t
    u[0] = 1.5
    print(u[0])
    local pick = {}
    if 1 > 2 then
        pick = {}
    else
        pick = u
    end
    pick[1] = 2.5
    print(pick[1], pick[0])
end
print(sys_alloc_count())
print("gauntlet complete")
