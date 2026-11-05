-- EXPECT: TABLE 17 (Phase N) FOLD	223608080032
-- EXPECT: TABLE 18 (Phase O) CHECKSUM	1333333330000
-- EXPECT: TABLE 19 (Phase Q) FAR CELL	123456789
-- EXPECT: TABLE 20 (Phase R) CHECKSUM	26086400000
-- EXPECT: TABLE 21 (Phase U) CHECKSUM	402380000
-- EXPECT: TABLE 22 (Phase Z) CHECKSUM	6004799503073280
local pn_board = {}
local pn_sq = 0
while pn_sq < 4096 do
    pn_board[pn_sq] = pn_sq * pn_sq - pn_sq
    pn_sq = pn_sq + 1
end
local pn_moves = 20000
local pn_move = 0
local pn_fold = 0
local pn_w1 = 0
local pn_w2 = 0
local pn_wa = 0
local pn_view = pn_board
while pn_move < pn_moves do
    local pn_attacks = {}
    local pn_from = 0
    while pn_from < 64 do
        pn_attacks[pn_from] = pn_from + pn_move
        pn_from = pn_from + 1
    end
    pn_w1 = pn_w1 + 17
    while 4095 < pn_w1 do pn_w1 = pn_w1 - 4096 end
    pn_w2 = pn_w2 + 41
    while 4095 < pn_w2 do pn_w2 = pn_w2 - 4096 end
    pn_wa = pn_wa + 7
    while 63 < pn_wa do pn_wa = pn_wa - 64 end
    pn_fold = pn_fold + pn_view[pn_w1] + pn_view[pn_w2] + pn_attacks[pn_wa]
    pn_attacks = nil
    pn_move = pn_move + 1
end
print("TABLE 17 (Phase N) FOLD", pn_fold)

local po_a = {}
local po_b = {}
local po_lim = 20000
local po_i = 0
while po_i < po_lim do
    local po_k = po_i + po_i
    po_a[po_k] = po_i
    po_b[po_k] = po_i + 1
    po_i = po_i + 1
end
local po_checksum = 0
local po_wit = 0
while po_wit < po_lim do
    po_checksum = po_checksum + (po_wit + 1) * (po_a[po_wit] + po_b[po_wit])
    po_wit = po_wit + 2
end
print("TABLE 18 (Phase O) CHECKSUM", po_checksum)

local pq_far = {}
pq_far[262143999] = 123456789
print("TABLE 19 (Phase Q) FAR CELL", pq_far[262143999])

local pr_tris = 20000
local pr_tri = 0
local pr_checksum = 0
while pr_tri < pr_tris do
    local pr_span = {}
    local pr_x = 0
    while pr_x < 128 do
        pr_span[pr_x] = pr_x * 3 + pr_tri
        pr_x = pr_x + 1
    end
    local pr_acc = 0
    local pr_j = 0
    while pr_j < 128 do
        pr_acc = pr_acc + pr_span[pr_j]
        pr_j = pr_j + 1
    end
    pr_checksum = pr_checksum + pr_acc
    pr_span = nil
    pr_tri = pr_tri + 1
end
print("TABLE 20 (Phase R) CHECKSUM", pr_checksum)

local pu_frames = 20000
local pu_f = 0
local pu_checksum = 0
while pu_f < pu_frames do
    local pu_parent = {}
    local pu_i = 0
    while pu_i < 16 do
        pu_parent[pu_i] = pu_i
        pu_i = pu_i + 1
    end
    pu_parent[3] = pu_parent[3] + pu_f
    pu_parent[7] = pu_parent[7] + pu_f
    local pu_acc = 0
    pu_i = 0
    while pu_i < 16 do
        pu_acc = pu_acc + pu_parent[pu_i]
        pu_i = pu_i + 1
    end
    pu_checksum = pu_checksum + pu_acc
    pu_parent = nil
    pu_f = pu_f + 1
end
print("TABLE 21 (Phase U) CHECKSUM", pu_checksum)

local pz_buf = {}
local pz_lim = 262144
local pz_i = 0
while pz_i < pz_lim do
    pz_buf[pz_i] = pz_i + 1
    pz_i = pz_i + 1
end
local pz_checksum = 0
local pz_w = 0
while pz_w < pz_lim do
    pz_checksum = pz_checksum + pz_w * pz_buf[pz_w]
    pz_w = pz_w + 1
end
print("TABLE 22 (Phase Z) CHECKSUM", pz_checksum)
