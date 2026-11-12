#!/usr/bin/env lua

local script_path = arg[0] or debug.getinfo(1, "S").source:match("^@(.+)$") or "."
local conf_dir = script_path:match("^(.+)[/\\]") or "."
dofile(conf_dir .. "/conf.lua")

local function usage()
    print([[
Usage: lua run.lua [SUBSTR ...] [FLAGS]
  (no args)  the full battery: corpus + exe twins + lock + fmt + clippy
  SUBSTR...  run only cases whose name contains any SUBSTR
  probe FILE diagnostic: one file through the same pipeline, archived
             under target/probe/ (skips the fmt/clippy legs)
Flags (any position):
  --cases DIR   external corpus, lockless by design — no notices
  --no-lock     skip the lock comparison (repo corpus)
  --relock      delegate to reset.lua (rewrite baselines), then exit;
                must stand alone
  --no-fmt      skip the rustfmt gate (a default, quiet leg)
  --no-clippy   skip the clippy gate (a default, quiet leg)
  --no-test    skip the cargo test gate (a zero-arg leg)
  --no-exe     skip the standalone-twin section
  --quick       corpus only (= --no-fmt --no-clippy --no-exe)
  --miri        opt-in extra leg: the lib unit tests under Miri
                (cargo +nightly miri test --lib — the rt.rs UB oracle;
                slow, needs the nightly toolchain with the component)

Quiet when green: one summary line; failures and action-needed notices
print as they happen; exit nonzero on any failure.]])
end

local ARGS = { ... }
local mode = "run"
local probe_file = nil
if ARGS[1] == "probe" then
    mode = "probe"
    probe_file = ARGS[2]
    if not probe_file then
        usage()
        io.stderr:write("\nprobe needs a FILE argument\n")
        os.exit(1)
    end
end

local NO_LOCK, NO_FMT, NO_CLIPPY, NO_EXE, NO_TEST, RELOCK, MIRI = false, false, false, false, false, false, false
local CASES_OVERRIDE = nil
local filters = {}
do
    local i = (mode == "probe") and 3 or 1
    while i <= #ARGS do
        local a = ARGS[i]
        if a == "--no-lock" then
            NO_LOCK = true
        elseif a == "--no-fmt" then
            NO_FMT = true
        elseif a == "--no-clippy" then
            NO_CLIPPY = true
        elseif a == "--no-test" then
            NO_TEST = true
        elseif a == "--no-exe" then
            NO_EXE = true
        elseif a == "--quick" then
            NO_FMT, NO_CLIPPY, NO_EXE, NO_TEST = true, true, true, true
        elseif a == "--miri" then
            MIRI = true
        elseif a == "--relock" then
            RELOCK = true
        elseif a == "--cases" then
            if i + 1 > #ARGS then
                io.stderr:write("\n--cases needs a directory argument\n")
                os.exit(1)
            end
            CASES_OVERRIDE = ARGS[i + 1]
            i = i + 1
        elseif a:sub(1, 2) == "--" then
            usage()
            io.stderr:write("\nunknown flag: " .. a .. "\n")
            os.exit(1)
        else
            table.insert(filters, a)
        end
        i = i + 1
    end
end

if RELOCK and (CASES_OVERRIDE or NO_LOCK or NO_FMT or NO_CLIPPY or NO_EXE or NO_TEST
               or MIRI or #filters > 0 or mode == "probe") then
    io.stderr:write("\n--relock must stand alone (it rewrites every baseline; verify after with a zero-arg run)\n")
    os.exit(1)
end

if CASES_OVERRIDE then
    local res = os.execute("test -d '" .. CASES_OVERRIDE .. "'")
    if not (res == 0 or res == true) then
        io.stderr:write("\n--cases: not a directory: " .. CASES_OVERRIDE .. "\n")
        os.exit(1)
    end
    CASES_DIR = CASES_OVERRIDE
end

if RELOCK then
    local res = os.execute("lua '" .. conf_dir .. "/reset.lua'")
    os.exit((res == 0 or res == true) and 0 or 1)
end

local GREEN, RED, YELLOW = "\27[32m", "\27[31m", "\27[33m"
local function c(code, s) return code .. s .. "\27[0m" end

local failed, notices = {}, {}
local function report_fail(section, name, reason)
    table.insert(failed, { section = section, name = name })
    print(string.format("  %s [%s] %s — %s", c(RED, "✗"), section, name, reason))
end
local function report_notice(text)
    table.insert(notices, text)
    print(string.format("  %s %s", c(YELLOW, "!"), text))
end

local function cap(s, n)
    local lines = {}
    for line in s:gmatch("[^\r\n]+") do table.insert(lines, line) end
    if #lines <= n then return s end
    return table.concat(lines, "\n", 1, n)
        .. string.format("\n       ... (%d more diff lines — full diff: %s vs %s)", #lines - n, DIFF_DIR, LOCK_DIR)
end

local function split_lines(s)
    local t = {}
    for line in s:gmatch("[^\r\n]+") do table.insert(t, line) end
    return t
end

-- EXPECT pins: whole-stdout equality — lines in order, nothing extra.
-- Only enforced when the case carries at least one EXPECT pin; a pinless
local function expect_mismatch(out, pins)
    local got = split_lines(out)
    for i = 1, math.max(#pins, #got) do
        local w, g = pins[i], got[i]
        if w == nil then
            return string.format("line %d: unexpected extra output '%s' (want %d lines, got %d)",
                i, g, #pins, #got)
        elseif g == nil then
            return string.format("line %d: want '%s', got nothing (want %d lines, got %d)",
                i, w, #pins, #got)
        elseif w ~= g then
            return string.format("line %d: want '%s', got '%s'", i, w, g)
        end
    end
    return nil
end

os.execute("rm -rf '" .. DIFF_DIR .. "'")
os.execute("mkdir -p '" .. DIFF_DIR .. "'")
os.execute("touch " .. DIFF_DIR .. "/.gitkeep")

local lock_identical, lock_drifted, lock_errored, lock_missing = 0, 0, 0, 0
local lock_missing_names = {}

local function lock_name(name) return name:gsub("%.lua$", "") .. ".ll" end

local function diff_against_lock(name, is_exe)
    if CASES_OVERRIDE then return nil end
    if NO_LOCK then return nil end
    local lock = LOCK_DIR .. "/" .. lock_name(name)
    if not io.open(lock, "r") then
        lock_missing = lock_missing + 1
        table.insert(lock_missing_names, name)
        return nil
    end
    local code = is_exe and captured_ir_exe() or captured_ir()
    if not code then
        lock_errored = lock_errored + 1
        report_fail("lock", name, "compile succeeded but the emitted .ll is empty/missing")
        return nil
    end
    -- Both sides normalized (conf.lua normalize_ir): old
    -- machine-absolute locks and post-normalization portable locks
    -- compare identically.
    local lockcode = normalize_ir(read_file(lock))
    local df = DIFF_DIR .. "/" .. lock_name(name)
    local f = io.open(df, "w")
    f:write(code)
    f:close()
    local lf = df .. ".fromlock"
    f = io.open(lf, "w")
    f:write(lockcode)
    f:close()
    local p = io.popen(string.format("diff -u '%s' '%s'", lf, df))
    local d = p:read("*a")
    p:close()
    if d == "" then
        lock_identical = lock_identical + 1
        return nil
    end
    lock_drifted = lock_drifted + 1
    report_fail("lock", name, "BYTE DRIFT vs " .. LOCK_DIR .. " — relock only at a milestone:\n" .. cap(d, 60))
    return nil
end

do
    local res = os.execute("cargo build --release --quiet > /dev/null 2> " .. BERR)
    if not (res == 0 or res == true) then
        print(c(RED, "  ✗ cargo build failed:") .. "\n" .. read_file(BERR))
        os.exit(1)
    end
end

if mode == "probe" then
    local ok = run_case(probe_file, nil, true)
    local compiled = compile_succeeded()
    local name = probe_file:gsub("%.lua$", ""):gsub("(.*/)", "")
    local dir = "target/probe"
    os.execute("mkdir -p " .. dir)
    if compiled then copy_file("libalm_out.ll", dir .. "/" .. name .. ".ll") end
    copy_file(".alm_trace.bin", dir .. "/" .. name .. ".bin")
    copy_file(".alm_rt_trace.bin", dir .. "/" .. name .. ".rt.bin")
    print("\n== PROBE " .. probe_file .. " ==")
    print("  compile: " .. (compiled and "ok" or ("FAILED — " .. first_error(ERR))))
    print("  exit   : " .. (ok and "0" or "nonzero"))
    local out = script_stdout()
    if out ~= "" then print("  stdout :\n" .. out) end
    print("  plates : " .. dir .. "/" .. name .. ".bin (.rt.bin sidecar"
        .. (compiled and ", .ll IR" or "") .. ") — decode: python3 plate.py <plate>")
    os.exit(0)
end

if not NO_FMT then
    local res = os.execute("cargo fmt --check > /dev/null 2> " .. BERR)
    if not (res == 0 or res == true) then
        print(c(RED, "  ✗ cargo fmt --check failed (run `cargo fmt` and re-stage):") .. "\n" .. read_file(BERR))
        os.exit(1)
    end
end
if not NO_CLIPPY then
    local res = os.execute("cargo clippy --release --quiet -- -D warnings > /dev/null 2> " .. BERR)
    if not (res == 0 or res == true) then
        print(c(RED, "  ✗ cargo clippy failed (-D warnings):") .. "\n" .. read_file(BERR))
        os.exit(1)
    end
end
-- The unit/property pins in src/ (rt.rs strkey_tests, fast_lane_facts_pin,
-- break_gate_tests, corpus_sweeps) ride the same battery as the .lua
-- corpus: a zero-arg run that skips them lets the lock suite drift from
-- the pins (the P20 lesson — the nested-row reserve pin was silently
-- broken for a session). Failure output lands in target/test_out.txt
-- (libtest's failure list prints on stdout, BERR only holds cargo's).
if not NO_TEST and #filters == 0 then
    local TEST_OUT = "target/test_out.txt"
    local res = os.execute("cargo test --quiet > " .. TEST_OUT .. " 2> " .. BERR)
    if not (res == 0 or res == true) then
        print(c(RED, "  ✗ cargo test failed (full output: " .. TEST_OUT .. "):")
            .. "\n" .. read_file(TEST_OUT) .. read_file(BERR))
        os.exit(1)
    end
end
-- The Miri leg, recorded so it stops being a commit-receipt ritual:
-- interprets the lib unit tests (the rt.rs UB oracle — poison reads
-- are cfg'd out, the ownership laws run interpreted). Opt-in because
-- it costs ~20s and a nightly toolchain; it tests the library, not
-- the corpus, so filters do not gate it.
if MIRI then
    local MIRI_OUT = "target/miri_out.txt"
    local res = os.execute("cargo +nightly miri test --lib --quiet > " .. MIRI_OUT .. " 2> " .. BERR)
    if not (res == 0 or res == true) then
        print(c(RED, "  ✗ miri failed (full output: " .. MIRI_OUT .. "):")
            .. "\n" .. read_file(MIRI_OUT) .. read_file(BERR))
        os.exit(1)
    end
end

local function selected(name)
    if #filters == 0 then return true end
    for _, s in ipairs(filters) do
        if name:find(s, 1, true) then return true end
    end
    return false
end

local corpus = scan_corpus()
local total = #corpus.positive + #corpus.negative + #corpus.panic
if #filters > 0 then
    local function keep(list)
        local t = {}
        for _, n in ipairs(list) do if selected(n) then table.insert(t, n) end end
        return t
    end
    corpus.positive = keep(corpus.positive)
    corpus.negative = keep(corpus.negative)
    corpus.panic = keep(corpus.panic)
end
for _, b in ipairs(corpus.bad) do report_fail("corpus", b[1], b[2]) end
for _, o in ipairs(corpus.odd) do
    report_notice(o[1] .. " — pin prefixes outside the grammar: " .. o[2])
end
local matched = #corpus.positive + #corpus.negative + #corpus.panic
if #filters > 0 and matched == 0 then
    io.stderr:write("\nfilter matched no cases: " .. table.concat(filters, " ") .. "\n")
    os.exit(1)
end

local pos_passed = 0
for _, name in ipairs(corpus.positive) do
    local pins = corpus.pins[name]
    -- --exe-flagged positives (the loading lane: ffi.load is
    -- --exe-only, M4's gate) compile the standalone executable and
    -- run it; callee-seam positives ride the lib lane as everything
    -- else does.
    if flags_exe(pins.flags) then
        if not run_case_exe(CASES_DIR .. "/" .. name, pins.flags)
            or not compile_succeeded_exe() then
            report_fail("positive", name, "compile failed: " .. first_error(ERR))
        else
            local argstr = pins.args and (" " .. pins.args) or ""
            if not run_exe_with(argstr) then
                report_fail("positive", name, "run failed: " .. first_error(ERR))
            else
                local bad
                if #pins.expect > 0 then
                    bad = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("positive", name, "EXPECT mismatch — " .. bad)
                else
                    pos_passed = pos_passed + 1
                end
            end
            diff_against_lock(name, true)
        end
    elseif not run_case(CASES_DIR .. "/" .. name, pins.args, false, pins.flags) then
        local why = compile_succeeded() and "run failed" or "compile failed"
        report_fail("positive", name, why .. ": " .. first_error(ERR))
    else
        diff_against_lock(name)
        local bad
        if #pins.expect > 0 then
            bad = expect_mismatch(script_stdout(), pins.expect)
        end
        if bad then
            report_fail("positive", name, "EXPECT mismatch — " .. bad)
        else
            pos_passed = pos_passed + 1
        end
    end
end

local neg_passed = 0
for _, name in ipairs(corpus.negative) do
    local pins = corpus.pins[name]
    run_case(CASES_DIR .. "/" .. name, pins.args, false, pins.flags)
    if compile_succeeded() then
        report_fail("negative", name, "expected compile failure, but it SUCCEEDED — the guard is gone")
    else
        local berr = read_file(ERR)
        local bad
        for _, msg in ipairs(pins.build_fail) do
            if not berr:find(msg, 1, true) then bad = "missing expected error: " .. msg break end
        end
        if bad then
            report_fail("negative", name, bad)
        else
            neg_passed = neg_passed + 1
        end
    end
end

local pan_passed = 0
for _, name in ipairs(corpus.panic) do
    local pins = corpus.pins[name]
    local died = not run_case(CASES_DIR .. "/" .. name, pins.args, true, pins.flags)
    if not compile_succeeded() then
        report_fail("panic", name, "expected compile to succeed: " .. first_error(ERR))
    else
        diff_against_lock(name)
        if not died then
            report_fail("panic", name, "expected a runtime death, but it ran clean")
        else
            local err = read_file(ERR)
            local bad
            for _, msg in ipairs(pins.panic) do
                if not err:find(msg, 1, true) then bad = "missing expected stderr: " .. msg break end
            end
            if not bad and #pins.expect > 0 then
                bad = expect_mismatch(script_stdout(), pins.expect)
            end
            if bad then
                report_fail("panic", name, bad)
            else
                pan_passed = pan_passed + 1
            end
        end
    end
end

if lock_missing > 0 then
    local lockables = #corpus.positive + #corpus.panic
    if #filters == 0 and lock_missing == lockables then
        report_notice(string.format(
            "no lock baseline on any lockable case (%d/%d) — fresh clone? run: lua run.lua --relock",
            lock_missing, lockables))
    else
        for _, name in ipairs(lock_missing_names) do
            report_notice(name .. " — no lock baseline (new case? relock at the next milestone)")
        end
    end
end

-- exec time). Positive cases must run clean and match their EXPECT
local exe_passed = 0
if not NO_EXE then
    local function exe_twin(name, must_die)
        if not run_case_exe(CASES_DIR .. "/" .. name, corpus.pins[name].flags) then
            report_fail("exe", name, "standalone compile failed: " .. first_error(ERR))
            return
        end
        if not compile_succeeded_exe() then
            report_fail("exe", name, "standalone compile failed: " .. first_error(ERR))
            return
        end
        local pins = corpus.pins[name]
        local argstr = pins.args and (" " .. pins.args) or ""
        local ok = run_exe_with(argstr)
        if must_die then
            if ok then
                report_fail("exe", name, "expected the scripted death inside the executable, it ran clean")
            else
                local err = read_file(ERR)
                local bad
                for _, msg in ipairs(pins.panic) do
                    if not err:find(msg, 1, true) then bad = "missing expected stderr: " .. msg break end
                end
                if not bad and #pins.expect > 0 then
                    bad = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("exe", name, bad)
                else
                    exe_passed = exe_passed + 1
                end
            end
        else
            if not ok then
                report_fail("exe", name, "executable exited nonzero: " .. first_error(ERR))
            else
                local bad
                if #pins.expect > 0 then
                    bad = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("exe", name, "EXPECT mismatch (exe) — " .. bad)
                else
                    exe_passed = exe_passed + 1
                end
            end
        end
    end
    for _, name in ipairs(corpus.positive) do exe_twin(name, false) end
    for _, name in ipairs(corpus.panic) do exe_twin(name, true) end
end

if #filters == 0 and not CASES_OVERRIDE and not NO_LOCK then
    local expected = {}
    for _, name in ipairs(corpus.positive) do expected[lock_name(name)] = true end
    for _, name in ipairs(corpus.panic) do expected[lock_name(name)] = true end
    local p = io.popen("ls " .. LOCK_DIR .. " 2>/dev/null")
    for line in p:lines() do
        if not expected[line] then
            report_notice("stray lock file (case deleted? lockdown removes orphans): " .. LOCK_DIR .. "/" .. line)
        end
    end
    p:close()
end

if #failed > 0 then
    print(c(RED, string.format("FAILURES : %d", #failed)))
    for _, f in ipairs(failed) do print(string.format("   [%s] %s", f.section, f.name)) end
    os.exit(1)
end
if #filters > 0 then
    print(c(YELLOW, string.format("  GREEN (filtered %d/%d by '%s') — a full zero-arg run is still owed",
        matched, total, table.concat(filters, "' '"))))
    os.exit(0)
end
local parts = { string.format("%d+%d+%d", pos_passed, neg_passed, pan_passed) }
if CASES_OVERRIDE then
    parts[1] = parts[1] .. " external (" .. CASES_DIR .. ")"
end
if not NO_EXE then
    table.insert(parts, string.format("%d exe twins", exe_passed))
end
if not CASES_OVERRIDE and not NO_LOCK then
    local compared = lock_identical + lock_drifted + lock_errored
    local lockpart = string.format("lock %d/%d", lock_identical, compared)
    if lock_missing > 0 then
        lockpart = lockpart .. string.format(" (+%d missing)", lock_missing)
    end
    table.insert(parts, lockpart)
end
local hygiene = {}
if not NO_TEST and #filters == 0 then table.insert(hygiene, "test") end
if MIRI then table.insert(hygiene, "miri") end
if not NO_FMT then table.insert(hygiene, "fmt") end
if not NO_CLIPPY then table.insert(hygiene, "clippy") end
if #hygiene > 0 then table.insert(parts, table.concat(hygiene, "+") .. " clean") end
print(c(GREEN, "  ALL GREEN — " .. table.concat(parts, " · ")))
