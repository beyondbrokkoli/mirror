-- P26 engine bring-up probe: memory_alloc_cpu.lua (the 32-line CPU
-- SoA allocator) against the dialect, W4 freshly landed. The W4 lane
-- (cdef/new/sizeof) compiles; every other rejection recorded verbatim
-- — this ladder is the W5+ roadmap's ground truth (memory_lane.md §7).
--
-- RECEIPTS (verbatim, in peel order):
--  1. require("runtime.services.memory.memory_base") — Syntax Error:
--     "the dialect provides only require("ffi") ... and
--     require("coroutine")" — the engine's require graph is the
--     module-system ask. [stubbed to proceed]
--  2. function AllocCPU.AllocateSoA(...) — Syntax Error: "expected a
--     name after '.'" — dotted fn defs; fn values bind to variables.
--  3. base_type .. "*" — Syntax Error: "expected a name after '.'" —
--     `..` concat is not in the dialect; the string kitchen's concat
--     op via the drive seam is the answer (interop/str_lib).
--  4. AllocCPU.AllocateSoA = function(...) — Type Error: "a function
--     value binds directly to a variable".
--  5. AllocCPU.AllocateSoA = local_fn — Type Error: "a closure value
--     enters only a record's literal fields" — the closed-vtable
--     inference milestone's first customer (SESSION §4.6).
--     [record-literal module table stubbed to proceed]
--  6. string.gsub(type_str, "%[.-%]", "") — Scope Error: "reference
--     to undeclared variable 'string'" — the string kitchen drive
--     seam (interop/str_lib/cbridge/string_kitchen.c).
--  7. Memory.AVX_Arrays[names[i]] — Scope Error: "dot access needs a
--     record or the ffi surface" — nested record-field chains need
--     the W5 row/record milestone.
--  8. ffi.sizeof(base_type) with a runtime string — Type Error:
--     "ffi.sizeof names its declaration by a string literal" — W4's
--     sizeof is registry-keyed; the engine's dynamic-name pattern
--     needs gsub const-folding or a declaration-name spelling. REAL
--     semantic gap, not a stub.
--  9. ffi.cast(base_type, raw_ptr) — Type Error: "unknown ffi member
--     'cast' — load, cdef, arg_kind, new, sizeof" — the typed-view
--     milestone (TDD §4.3), stub rejection per plan.
-- 10. assert(...) — Type Error: "'assert' is not an expansion" —
--     assert not in the dialect.
--
-- Compiles clean in the W4 lane: numeric-for, #names, table params,
-- record-literal module tables with closure fields, print.
local Memory = { AVX_Arrays = {} }
local Platform = {}

local ffi = require("ffi")


local function allocate(type_str, count, names)
    local base_type = string.gsub(type_str, "%[.-%]", "")
    local byte_size = ffi.sizeof(base_type) * count
    local align_bytes = 32

    for i = 1, #names do
        local raw_ptr = Platform.aligned_alloc(align_bytes, byte_size)
        assert(raw_ptr ~= nil, "FATAL: C-Allocator failed to provide aligned memory!")
        Memory.AVX_Arrays[names[i]] = ffi.cast(base_type, raw_ptr)
        print(string.format("[MEMORY] Allocated Fast CPU RAM: %s (%.2f MB)", names[i], byte_size / (1024*1024)))
    end
end

local function free_soa(names)
    for i = 1, #names do
        local ptr = Memory.AVX_Arrays[names[i]]
        if ptr then
            Platform.aligned_free(ptr)
            Memory.AVX_Arrays[names[i]] = nil
        end
    end
end

local AllocCPU = { AllocateSoA = allocate, FreeSoA = free_soa }

return AllocCPU
