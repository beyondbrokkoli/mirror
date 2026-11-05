-- EXPECT: 4000000	3999999	-7780081407043551616
-- EXPECT: 45	50
-- EXPECT: 2499999	6666666666
-- EXPECT: 1	30	60
-- EXPECT: 7	777
-- EXPECT: 60000	59999
-- EXPECT: 60000	1800030000
-- EXPECT: 180	136	91
-- EXPECT: 250	31375
-- EXPECT: 100
-- EXPECT: 49	150	0
-- EXPECT: 3	8
-- EXPECT: 5

local pa = {}
local pa_lim = 4000000
local pa_i = 0
while pa_i < pa_lim do
    pa[pa_i] = pa_i + 1
    pa_i = pa_i + 2
end
local pa_sum = 0
pa_i = 0
while pa_i < pa_lim do
    pa_sum = pa_sum + pa_i * pa[pa_i]
    pa_i = pa_i + 1
end
print(pa_lim, pa[3999998], pa_sum)

local pb = {}
local pb_lim = 100
local pb_i = 0
while pb_i < pb_lim do
    local pb_j = 0
    while pb_j < pb_i do
        pb[pb_j] = pb_j + 1
        pb_j = pb_j + 1
    end
    pb_i = pb_i + 1
end
print(pb[44], pb[49])

local pc = {}
local pc_lim = 2500000
local pc_i = 0
while pc_i < pc_lim do
    pc[pc_i] = 5
    pc_i = pc_i + 1
end
local pc_j = 0
while pc_j < pc_lim do
    pc[pc_j] = pc[pc_j] + 1
    pc_j = pc_j + 1
end
local pc_acc = 0
pc_i = 0
while pc_i < 10 do
    pc_acc = pc_acc * 10 + pc[pc_i]
    pc_i = pc_i + 1
end
print(pc_lim - 1, pc_acc)

local pd = {}
local pd_i = 0
while pd_i < 30000 do
    pd[pd_i] = 1
    pd_i = pd_i + 1
end
pd[30] = 60
local pd_bound = 60
local pd_s = 0
while pd[pd_s] < pd_bound do
    pd_s = pd_s + 1
end
print(pd[0], pd_s, pd[30])

local pe = {}
local pe_lim = 200000
local pe_horizon = 3
local pe_i = 0
while pe_i < pe_lim do
    local pe_frozen = pe_i
    local pe_j = 0
    while pe_j < pe_horizon do
        pe[pe_frozen] = 7
        pe_j = pe_j + 1
    end
    pe_i = pe_i + 1
end
local pe_s = 0
pe_i = 0
while pe_i < 3 do
    pe_s = pe_s * 10 + pe[pe_i]
    pe_i = pe_i + 1
end
print(pe[2], pe_s)

local pf = {}
local pf_lim = 60000
local pf_i = 0
while pf_i < pf_lim do
    local pf_r = pf_i
    while pf_r < pf_lim do
        pf[pf_r] = pf_r + 1
        pf_r = pf_r + 1
    end
    pf_i = pf_i + 1
end
print(pf[59999], pf[59998])
local pf_sum = 0
pf_i = 0
while pf_i < pf_lim do
    pf_sum = pf_sum + pf[pf_i]
    pf_i = pf_i + 1
end
print(pf_lim, pf_sum)

local pg_src = {}
local pg_dst = {}
local pg_lim = 180
local pg_i = 0
while pg_i < pg_lim do
    pg_src[pg_i] = pg_i + 1
    pg_i = pg_i + 1
end
local pg_j = 0
while pg_j < pg_lim do
    local pg_m = pg_lim - 1 - pg_j
    pg_dst[pg_m] = pg_src[pg_j]
    pg_j = pg_j + 1
end
print(pg_dst[0], pg_dst[44], pg_dst[89])

local ph = {}
local ph_lim = 250
local ph_i = 0
while ph_i < ph_lim do
    ph[ph_i] = ph_i + 1
    ph_i = ph_i + 1
end
local ph_acc = 0
ph_i = 0
while ph_i < ph_lim do
    ph_acc = ph_acc + ph[ph_i]
    ph_i = ph_i + 1
end
print(ph_lim, ph_acc)

local pi = {}
local pi_vibing = 0 < 1
local pi_i = 0
while pi_vibing do
    pi[pi_i] = 100
    pi_vibing = 0 < 0
    pi_i = pi_i + 1
end
print(pi[0])

local pj = {}
local pj_i = 100
while pj_i < 150 do
    pj[pj_i] = pj_i - 100
    pj_i = pj_i + 1
end
local pj_j = 150
while pj_j < 200 do
    pj[pj_j] = pj_j
    pj_j = pj_j + 1
end
print(pj[149], pj[150], pj[99])

local pl = {}
local pl_lim = 8
local pl_i = 0
while pl_i < pl_lim do
    local pl_a = pl_i
    pl[pl_a] = 1
    local pl_b = pl_a
    pl_b = pl_b + 0
    pl[pl_b] = 2
    local pl_g = pl_i
    pl[pl_g] = 3
    pl_i = pl_i + 1
end
print(pl[7], pl_lim)

local pm_a = {}
local pm_lim = 8
local pm_i = 0
while pm_i < pm_lim do
    pm_a[pm_i] = pm_i + 2
    pm_i = pm_i + 1
end
print(pm_a[3])
