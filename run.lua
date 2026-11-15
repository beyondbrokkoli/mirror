#!/usr/bin/env lua

local script_path = arg[0] or debug.getinfo(1, "S").source:match("^@(.+)$") or "."
local conf_dir = script_path:match("^(.+)[/\\]") or "."
dofile(conf_dir .. "/conf.lua")

local function usage()
    print([[
Usage: lua run.lua [SUBSTR ...] [FLAGS]
  (no args)  the full battery: fmt + build + clippy + test + corpus + exe twins + lock
  SUBSTR...  run only cases whose name contains any SUBSTR
  probe FILE diagnostic: one file through the same pipeline, archived
             under target/probe/ (skips the fmt/clippy/test legs)
Leak gate    positive runs (lib lane and exe twins) also gate on the
             runtime's slot-90 atexit notice (the zero-leak contract);
             panic lanes skip it — a death mid-flight may leave its
             tables. The notice rides stderr, which the stdout twins
             never see.
XFAIL pin    -- XFAIL: <reason> marks a case whose gate failures are
             expected today: the pin states the behavior that should
             hold once the named law lands, and the envelope keeps
             the case visible (dim lines, archived evidence) without
             failing the run. When every gate comes back green, the
             summary nags — flip the pin and relock. Runtime-
             aspirational only: an xfail positive must compile today
             (relock aborts otherwise); a guard that should reject a
             compile rides the negative lane, where xfail means "the
             guard does not fire yet".
Failures     every gate failure prints one specific line and archives
             the evidence (stdout, stderr, IR, lock diff, want/got)
             under target/failures/<case>.<lane>/ — the console never
             dumps diffs. Full reasons and the envelope census:
             target/failures/index.txt (rewritten each run).
Flags (any position):
  --cases DIR   external corpus, lockless by design — no notices
  --no-lock     skip the lock comparison (repo corpus)
  --relock      delegate to reset.lua (rewrite baselines), then exit;
                must stand alone
  --no-fmt      skip the rustfmt gate (a default, quiet leg)
  --no-clippy   skip the clippy gate (a default, quiet leg)
  --no-test     skip the cargo test gate (a zero-arg leg)
  --no-exe      skip the standalone-twin section
  --quick       corpus only (= --no-fmt --no-clippy --no-exe --no-test)
  --miri        opt-in extra leg: the lib unit tests under Miri
                (cargo +nightly miri test --lib — the rt.rs UB oracle;
                slow, needs the nightly toolchain with the component)

Locks (lock/ is untracked — the .ll baselines are wordy):
  fresh clone  no baselines: bootstrap once with --relock
  every run    the drift observatory — fresh IR byte-compared against
               lock/; drift means a refactor changed program behaviour.
               The locks are meant to drift when something legitimately
               changes; the observatory makes the change visible
  --relock     the authoring overwrite: rewrite baselines from this
               build, on wanted drift (a deliberate change, fix or
               improvement) or to give new cases a first baseline. A
               green run right after is a self-check, not a receipt —
               the baselines came from the same build (the summary
               names the lock origin)

Quiet when green: one summary line; failures and action-needed notices
print one line each as they happen; exit nonzero on any unexpected
failure (xfail envelopes don't fail the run).]])
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
    local ok = exec_cmd("test -d '" .. CASES_OVERRIDE .. "'")
    if not ok then
        io.stderr:write("\n--cases: not a directory: " .. CASES_OVERRIDE .. "\n")
        os.exit(1)
    end
    CASES_DIR = CASES_OVERRIDE
end

if RELOCK then
    local ok = exec_cmd("lua '" .. conf_dir .. "/reset.lua'")
    os.exit(ok and 0 or 1)
end

local GREEN, RED, YELLOW = "\27[32m", "\27[31m", "\27[33m"
local function c(code, s) return code .. s .. "\27[0m" end

local failed, notices = {}, {}
-- The case context: the lane loops set it while a case's gates run and
-- clear it after. report_fail consults it twice — an xfail case's gate
-- failures route into its envelope (expected, dim, never fatal), and
-- every run-born failure archives its evidence before the next case
-- overwrites OUT/ERR. Classification failures (corpus) and post-run
-- sweeps (missing/stray locks) run outside any case context.
local current_case = nil
local xfail_envelopes = {}  -- case name -> { {gate=, reason=}, ... }
local function oneline(s, n)
    local first = s:gsub("[\r\n].*", "")
    if #first > (n or 200) then
        first = first:sub(1, n or 200) .. "…"
    end
    return first
end
local function report_fail(section, name, reason, extras)
    local dir = nil
    if current_case and current_case.xfail then
        local env = xfail_envelopes[name]
        if not env then env = {}; xfail_envelopes[name] = env end
        table.insert(env, { gate = section, reason = reason })
        dir = archive_failure(name, section .. ".xfail", reason, extras)
        print(string.format("  · [xfail] %s %s — %s — %s", section, name,
            oneline(reason), dir))
        return
    end
    if current_case or section == "corpus" then
        dir = archive_failure(name, section, reason, extras)
    end
    table.insert(failed, { section = section, name = name, reason = reason, dir = dir })
    local where = dir and (" — " .. dir) or ""
    print(string.format("  %s [%s] %s — %s%s", c(RED, "✗"), section, name,
        oneline(reason), where))
end
local function report_notice(text)
    table.insert(notices, text)
    print(string.format("  %s %s", c(YELLOW, "!"), text))
end

-- Cargo-leg heads: the legs keep their full output in target/ files
-- that persist across runs; the console shows the head and names the
-- file, so a 200-line libtest dump never drowns the corpus failures
-- behind it.
local function leg_dump(path, n)
    local lines = split_lines(read_file(path))
    local head = {}
    for i = 1, math.min(n or 15, #lines) do head[i] = lines[i] end
    local s = table.concat(head, "\n")
    if #lines > (n or 15) then
        s = s .. string.format("\n       … (%d more lines — full output: %s)",
            #lines - (n or 15), path)
    end
    return s
end

-- EXPECT pins: whole-stdout equality — lines in order, nothing extra.
-- Only enforced when the case carries at least one EXPECT pin; a pinless
-- positive case only checks for a clean compile and zero exit status.
-- ALL differing lines are collected: the console shows the first three
-- (one iterate-run per mismatch line is a volume tax), the archive's
-- want.txt/got.txt hold both sides whole.
local function expect_mismatch(out, pins)
    local got = split_lines(out)
    local bad = {}
    for i = 1, math.max(#pins, #got) do
        local w, g = pins[i], got[i]
        if w == nil then
            table.insert(bad, string.format("line %d: unexpected extra output '%s' (want %d lines, got %d)",
                i, g, #pins, #got))
        elseif g == nil then
            table.insert(bad, string.format("line %d: want '%s', got nothing (want %d lines, got %d)",
                i, w, #pins, #got))
        elseif w ~= g then
            table.insert(bad, string.format("line %d: want '%s', got '%s'", i, w, g))
        end
    end
    if #bad == 0 then return nil end
    local shown = {}
    for i = 1, math.min(3, #bad) do shown[i] = bad[i] end
    local s = table.concat(shown, " | ")
    if #bad > 3 then
        s = s .. string.format(" | … (%d more)", #bad - 3)
    end
    return s, table.concat(pins, "\n"), table.concat(got, "\n")
end

os.execute("rm -rf '" .. DIFF_DIR .. "'")
os.execute("mkdir -p '" .. DIFF_DIR .. "'")
os.execute("touch '" .. DIFF_DIR .. "/.gitkeep'")

local lock_identical, lock_drifted, lock_errored, lock_missing = 0, 0, 0, 0
local lock_missing_names = {}

local function lock_name(name) return name:gsub("%.lua$", "") .. ".ll" end

local function diff_against_lock(name, is_exe)
    if CASES_OVERRIDE then return nil end
    if NO_LOCK then return nil end
    local lock = LOCK_DIR .. "/" .. lock_name(name)
    local lf_check = io.open(lock, "r")
    if not lf_check then
        lock_missing = lock_missing + 1
        table.insert(lock_missing_names, name)
        return nil
    end
    lf_check:close()
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
    -- The console never dumps the diff: one line names the size and
    -- the first hunk, the archive holds the whole unified diff plus
    -- both normalized sides. An xfail case's drift rides its envelope
    -- (the pin covers the case's whole gate set), so the observatory's
    -- counter stays honest about UNexpected drift.
    local first_hunk = d:match("(@@[^\n]*)")
    local short = string.format(
        "BYTE DRIFT vs %s: %d diff lines, first hunk %s — relock only at a milestone",
        LOCK_DIR, select(2, d:gsub("\n", "")) + 1, first_hunk or "(no hunk)")
    if not (current_case and current_case.xfail) then
        lock_drifted = lock_drifted + 1
    end
    report_fail("lock", name, short, {
        ["lock-diff.txt"] = d,
        ["lock-from.txt"] = lockcode,
        ["lock-to.txt"] = code,
    })
    return nil
end

-- Check formatting first (fast fail before spending time compiling)
if mode ~= "probe" and not NO_FMT then
    local ok = exec_cmd("cargo fmt --check > /dev/null 2> " .. BERR)
    if not ok then
        print(c(RED, "  ✗ cargo fmt --check failed (run `cargo fmt` and re-stage):"))
        print(leg_dump(BERR))
        os.exit(1)
    end
end

do
    local ok = exec_cmd("cargo build --release --quiet > /dev/null 2> " .. BERR)
    if not ok then
        print(c(RED, "  ✗ cargo build failed:"))
        print(leg_dump(BERR))
        os.exit(1)
    end
end

if mode == "probe" then
    local f = io.open(probe_file, "r")
    if not f then
        io.stderr:write("\nprobe: file not found: " .. probe_file .. "\n")
        os.exit(1)
    end
    f:close()
    local pins = parse_pins(read_file(probe_file))
    local is_exe = flags_exe(pins.flags)
    local ok, timed_out, compiled
    if is_exe then
        local comp_ok, comp_to = run_case_exe(probe_file, pins.flags)
        compiled = comp_ok and compile_succeeded_exe()
        if compiled then
            local argstr = pins.args and (" " .. pins.args) or ""
            ok, timed_out = run_exe_with(argstr, true)
        else
            ok, timed_out = false, comp_to
        end
    else
        ok, timed_out = run_case(probe_file, pins.args, true, pins.flags)
        compiled = compile_succeeded()
    end
    local name = probe_file:gsub("%.lua$", ""):gsub("(.*/)", "")
    local dir = "target/probe"
    os.execute("mkdir -p " .. dir)
    os.remove(dir .. "/" .. name .. ".ll")
    os.remove(dir .. "/" .. name .. ".bin")
    os.remove(dir .. "/" .. name .. ".rt.bin")
    if compiled then
        local ir_src = is_exe and "alm_out.ll" or "libalm_out.ll"
        copy_file(ir_src, dir .. "/" .. name .. ".ll")
    end
    local has_bin = copy_file(".alm_trace.bin", dir .. "/" .. name .. ".bin")
    local has_rt = copy_file(".alm_rt_trace.bin", dir .. "/" .. name .. ".rt.bin")
    print("\n== PROBE " .. probe_file .. " ==")
    print("  compile: " .. (compiled and "ok" or ("FAILED — " .. first_error(ERR, timed_out))))
    print("  exit   : " .. (timed_out and "TIMEOUT" or (ok and "0" or "nonzero")))
    local out = script_stdout()
    if out ~= "" then print("  stdout :\n" .. out) end
    local artifacts = {}
    if has_bin then table.insert(artifacts, dir .. "/" .. name .. ".bin") end
    if has_rt then table.insert(artifacts, ".rt.bin sidecar") end
    if compiled then table.insert(artifacts, ".ll IR") end
    if #artifacts > 0 then
        print("  plates : " .. table.concat(artifacts, ", ") .. " — decode: python3 plate.py <plate>")
    else
        print("  plates : (none emitted)")
    end
    clean_artifacts()
    os.exit(0)
end

-- The failure archive: wiped after probe mode exits (a probe must not
-- cost the previous run's evidence) and fresh for THIS run, like diff/.
os.execute("rm -rf '" .. FAIL_DIR .. "'")
os.execute("mkdir -p '" .. FAIL_DIR .. "'")

if not NO_CLIPPY then
    local ok = exec_cmd("cargo clippy --release --quiet -- -D warnings > /dev/null 2> " .. BERR)
    if not ok then
        print(c(RED, "  ✗ cargo clippy failed (-D warnings):"))
        print(leg_dump(BERR))
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
    local ok = exec_cmd("cargo test --release --quiet > " .. TEST_OUT .. " 2> " .. BERR)
    if not ok then
        print(c(RED, "  ✗ cargo test failed (full output: " .. TEST_OUT .. "):"))
        print(leg_dump(TEST_OUT))
        if read_file(BERR) ~= "" then print(leg_dump(BERR)) end
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
    local ok = exec_cmd("cargo +nightly miri test --lib --quiet > " .. MIRI_OUT .. " 2> " .. BERR)
    if not ok then
        print(c(RED, "  ✗ miri failed (full output: " .. MIRI_OUT .. "):"))
        print(leg_dump(MIRI_OUT))
        if read_file(BERR) ~= "" then print(leg_dump(BERR)) end
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
    report_fail("corpus", o[1], "unknown EXPECT_* pin prefix outside grammar: " .. o[2])
end
local matched = #corpus.positive + #corpus.negative + #corpus.panic
if #filters > 0 and matched == 0 then
    io.stderr:write("\nfilter matched no cases: " .. table.concat(filters, " ") .. "\n")
    os.exit(1)
end

local pos_passed = 0
for _, name in ipairs(corpus.positive) do
    local pins = corpus.pins[name]
    current_case = { name = name, pins = pins, xfail = pins.xfail }
    -- --exe-flagged positives (the loading lane: ffi.load is
    -- --exe-only, M4's gate) compile the standalone executable and
    -- run it; callee-seam positives ride the lib lane as everything
    -- else does.
    if flags_exe(pins.flags) then
        local comp_ok, comp_to = run_case_exe(CASES_DIR .. "/" .. name, pins.flags)
        if not comp_ok or not compile_succeeded_exe() then
            report_fail("positive", name, "compile failed: " .. first_error(ERR, comp_to))
        else
            diff_against_lock(name, true)
            local argstr = pins.args and (" " .. pins.args) or ""
            local run_ok, run_to = run_exe_with(argstr, false)
            if not run_ok then
                report_fail("positive", name, "run failed: " .. first_error(ERR, run_to))
            else
                local bad, want, got
                if #pins.expect > 0 then
                    bad, want, got = expect_mismatch(script_stdout(), pins.expect)
                end
                local leak = leak_notice()
                if bad then
                    report_fail("positive", name, "EXPECT mismatch — " .. bad,
                        { ["want.txt"] = want, ["got.txt"] = got })
                end
                if leak then
                    report_fail("leak", name, leak)
                end
                if not bad and not leak then
                    pos_passed = pos_passed + 1
                end
            end
        end
    else
        local ok, timed_out = run_case(CASES_DIR .. "/" .. name, pins.args, false, pins.flags)
        if not ok or not compile_succeeded() then
            local why = compile_succeeded() and "run failed" or "compile failed"
            report_fail("positive", name, why .. ": " .. first_error(ERR, timed_out))
        else
            diff_against_lock(name, false)
            local bad, want, got
            if #pins.expect > 0 then
                bad, want, got = expect_mismatch(script_stdout(), pins.expect)
            end
            local leak = leak_notice()
            if bad then
                report_fail("positive", name, "EXPECT mismatch — " .. bad,
                    { ["want.txt"] = want, ["got.txt"] = got })
            end
            if leak then
                report_fail("leak", name, leak)
            end
            if not bad and not leak then
                pos_passed = pos_passed + 1
            end
        end
    end
    current_case = nil
end

local neg_passed = 0
for _, name in ipairs(corpus.negative) do
    local pins = corpus.pins[name]
    current_case = { name = name, pins = pins, xfail = pins.xfail }
    local is_exe = flags_exe(pins.flags)
    local _, timed_out
    local succeeded
    if is_exe then
        _, timed_out = run_case_exe(CASES_DIR .. "/" .. name, pins.flags)
        succeeded = compile_succeeded_exe()
    else
        _, timed_out = run_case(CASES_DIR .. "/" .. name, pins.args, false, pins.flags)
        succeeded = compile_succeeded()
    end
    if timed_out then
        report_fail("negative", name, "compiler timed out (120s)")
    elseif succeeded then
        report_fail("negative", name, "expected compile failure, but it SUCCEEDED — the guard is gone")
    elseif compiler_panicked() then
        report_fail("negative", name, "compiler panicked (ICE) instead of emitting a diagnostic: " .. first_error(ERR))
    else
        local berr = read_file(ERR)
        local bad
        for _, msg in ipairs(pins.build_fail) do
            if not berr:find(msg, 1, true) then
                bad = "missing expected error: " .. msg
                break
            end
        end
        if bad then
            report_fail("negative", name, bad)
        else
            neg_passed = neg_passed + 1
        end
    end
    current_case = nil
end

local pan_passed = 0
for _, name in ipairs(corpus.panic) do
    local pins = corpus.pins[name]
    current_case = { name = name, pins = pins, xfail = pins.xfail }
    local is_exe = flags_exe(pins.flags)
    if is_exe then
        local comp_ok, comp_to = run_case_exe(CASES_DIR .. "/" .. name, pins.flags)
        if not comp_ok or not compile_succeeded_exe() then
            report_fail("panic", name, "expected compile to succeed: " .. first_error(ERR, comp_to))
        else
            diff_against_lock(name, true)
            local argstr = pins.args and (" " .. pins.args) or ""
            local run_ok, run_to = run_exe_with(argstr, true)
            if run_to then
                report_fail("panic", name, "runtime timed out (120s) instead of panicking")
            elseif run_ok then
                report_fail("panic", name, "expected a runtime death, but it ran clean")
            else
                local err = read_file(ERR)
                local bad
                for _, msg in ipairs(pins.panic) do
                    if not err:find(msg, 1, true) then
                        bad = "missing expected stderr: " .. msg
                        break
                    end
                end
                local want, got
                if not bad and #pins.expect > 0 then
                    bad, want, got = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("panic", name, bad, { ["want.txt"] = want, ["got.txt"] = got })
                else
                    pan_passed = pan_passed + 1
                end
            end
        end
    else
        local ok, timed_out = run_case(CASES_DIR .. "/" .. name, pins.args, true, pins.flags)
        if not compile_succeeded() then
            report_fail("panic", name, "expected compile to succeed: " .. first_error(ERR, timed_out))
        else
            diff_against_lock(name, false)
            if timed_out then
                report_fail("panic", name, "runtime timed out (120s) instead of panicking")
            elseif ok then
                report_fail("panic", name, "expected a runtime death, but it ran clean")
            else
                local err = read_file(ERR)
                local bad
                for _, msg in ipairs(pins.panic) do
                    if not err:find(msg, 1, true) then
                        bad = "missing expected stderr: " .. msg
                        break
                    end
                end
                local want, got
                if not bad and #pins.expect > 0 then
                    bad, want, got = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("panic", name, bad, { ["want.txt"] = want, ["got.txt"] = got })
                else
                    pan_passed = pan_passed + 1
                end
            end
        end
    end
    current_case = nil
end

if lock_missing > 0 then
    local lockables = #corpus.positive + #corpus.panic
    if #filters == 0 and lock_missing == lockables then
        report_fail("lock", "all", string.format(
            "no lock baseline on any lockable case (%d/%d) — run: lua run.lua --relock",
            lock_missing, lockables))
    else
        for _, name in ipairs(lock_missing_names) do
            if #filters == 0 then
                report_fail("lock", name, "missing lock baseline (run `lua run.lua --relock` or pass `--no-lock`)")
            else
                report_notice(name .. " — no lock baseline (new case? relock at the next milestone)")
            end
        end
    end
end

-- Standalone executable twins: every buildable case (positive and panic) must
-- also compile and behave identically when linked as a standalone executable
-- (--exe). Cases that already specify --exe in FLAGS were tested in the main
-- loop above and are counted without redundant recompilation.
local exe_passed = 0
if not NO_EXE then
    local function exe_twin(name, must_die)
        local pins = corpus.pins[name]
        if flags_exe(pins.flags) then
            -- Already ran via the standalone --exe lane in the primary loop
            exe_passed = exe_passed + 1
            return
        end
        local comp_ok, comp_to = run_case_exe(CASES_DIR .. "/" .. name, pins.flags)
        if not comp_ok or not compile_succeeded_exe() then
            report_fail("exe", name, "standalone compile failed: " .. first_error(ERR, comp_to))
            return
        end
        local argstr = pins.args and (" " .. pins.args) or ""
        local ok, run_to = run_exe_with(argstr, must_die)
        if run_to then
            report_fail("exe", name, "executable timed out (120s)")
            return
        end
        if must_die then
            if ok then
                report_fail("exe", name, "expected the scripted death inside the executable, it ran clean")
            else
                local err = read_file(ERR)
                local bad
                for _, msg in ipairs(pins.panic) do
                    if not err:find(msg, 1, true) then
                        bad = "missing expected stderr: " .. msg
                        break
                    end
                end
                local want, got
                if not bad and #pins.expect > 0 then
                    bad, want, got = expect_mismatch(script_stdout(), pins.expect)
                end
                if bad then
                    report_fail("exe", name, bad, { ["want.txt"] = want, ["got.txt"] = got })
                else
                    exe_passed = exe_passed + 1
                end
            end
        else
            if not ok then
                report_fail("exe", name, "executable exited nonzero: " .. first_error(ERR, run_to))
            else
                local bad, want, got
                if #pins.expect > 0 then
                    bad, want, got = expect_mismatch(script_stdout(), pins.expect)
                end
                local leak = leak_notice()
                if bad then
                    report_fail("exe", name, "EXPECT mismatch (exe) — " .. bad,
                        { ["want.txt"] = want, ["got.txt"] = got })
                end
                if leak then
                    report_fail("leak", name, leak)
                end
                if not bad and not leak then
                    exe_passed = exe_passed + 1
                end
            end
        end
    end
    for _, name in ipairs(corpus.positive) do
        current_case = { name = name, pins = corpus.pins[name], xfail = corpus.pins[name].xfail }
        exe_twin(name, false)
        current_case = nil
    end
    for _, name in ipairs(corpus.panic) do
        current_case = { name = name, pins = corpus.pins[name], xfail = corpus.pins[name].xfail }
        exe_twin(name, true)
        current_case = nil
    end
end

if #filters == 0 and not CASES_OVERRIDE and not NO_LOCK then
    local expected = {}
    for _, name in ipairs(corpus.positive) do expected[lock_name(name)] = true end
    for _, name in ipairs(corpus.panic) do expected[lock_name(name)] = true end
    local p = io.popen("ls '" .. LOCK_DIR .. "' 2>/dev/null")
    for line in p:lines() do
        if not expected[line] then
            report_fail("lock", line, "stray lock file in " .. LOCK_DIR .. "/ (case deleted? run `lua run.lua --relock`)")
        end
    end
    p:close()
end

clean_artifacts()

-- XPASS: an xfail pin whose whole gate envelope came back empty — the
-- law the pin was waiting for has landed. Evaluated after every lane
-- ran, so a lib-pass + exe-fail split cannot false-positive. Gentle:
-- the nag names the flip, the run stays green, the exit code is
-- untouched.
local xpass = {}
for _, list in ipairs({ corpus.positive, corpus.negative, corpus.panic }) do
    for _, name in ipairs(list) do
        local pins = corpus.pins[name]
        if pins.xfail and not xfail_envelopes[name] then
            table.insert(xpass, { name = name, reason = pins.xfail })
        end
    end
end
local xfail_n = 0
for _ in pairs(xfail_envelopes) do xfail_n = xfail_n + 1 end

-- The run's failure census, written before the verdicts: full,
-- untruncated reasons plus the envelope contents — the console shows
-- one line per event, this file holds the whole story.
if #failed > 0 or xfail_n > 0 or #xpass > 0 then
    local idx = io.open(FAIL_DIR .. "/index.txt", "w")
    if idx then
        for _, f in ipairs(failed) do
            idx:write(string.format("FAIL  [%s] %s\n  %s\n%s\n\n", f.section, f.name,
                oneline(f.reason, 400), f.dir and ("  evidence: " .. f.dir) or ""))
        end
        for name, env in pairs(xfail_envelopes) do
            idx:write(string.format("XFAIL %s\n  pin: %s\n", name,
                oneline(corpus.pins[name].xfail, 400)))
            for _, e in ipairs(env) do
                idx:write(string.format("  gate %s: %s\n", e.gate, oneline(e.reason, 400)))
            end
            idx:write("\n")
        end
        for _, x in ipairs(xpass) do
            idx:write(string.format("XPASS %s\n  pin: %s — flip and relock\n\n", x.name, x.reason))
        end
        idx:close()
    end
end

if #xpass > 0 then
    print(c(YELLOW, string.format("  ! XPASS — %d xfail pin%s retired-ready — flip the pin and relock:",
        #xpass, #xpass == 1 and "" or "s")))
    for _, x in ipairs(xpass) do
        print(string.format("      %s — pin was: %s", x.name, oneline(x.reason)))
    end
end

if #failed > 0 then
    print(c(RED, string.format("FAILURES : %d", #failed)))
    for _, f in ipairs(failed) do
        if f.dir then
            print(string.format("   [%s] %s — %s", f.section, f.name, f.dir))
        else
            print(string.format("   [%s] %s", f.section, f.name))
        end
    end
    os.exit(1)
end
if #filters > 0 then
    local extra = {}
    if xfail_n > 0 then table.insert(extra, string.format("xfail %d", xfail_n)) end
    if #xpass > 0 then table.insert(extra, string.format("%d xpass flip-ready", #xpass)) end
    print(c(YELLOW, string.format("  GREEN (filtered %d/%d by '%s') — a full zero-arg run is still owed%s",
        matched, total, table.concat(filters, "' '"),
        (#extra > 0) and (" · " .. table.concat(extra, " · ")) or "")))
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
    -- Origin provenance: reset.lua stamps lock/.origin. A green run at
    -- the same commit compared the build against itself — say so, so a
    -- self-check never poses as a drift receipt.
    local origin = read_file(LOCK_DIR .. "/.origin")
    local ohash, odate = origin:match("^(%S+) locked (.-)%s*$")
    if ohash then
        local p = io.popen("git rev-parse --short HEAD 2>/dev/null")
        local head = p:read("*l")
        p:close()
        if head and ohash == head then
            lockpart = lockpart
                .. string.format(" (origin = HEAD %s — self-check, not a receipt)", ohash)
        else
            lockpart = lockpart .. string.format(" (origin %s, %s)", ohash, odate)
        end
    end
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
if xfail_n > 0 then table.insert(parts, string.format("xfail %d (enveloped)", xfail_n)) end
if #xpass > 0 then table.insert(parts, string.format("%d xpass flip-ready", #xpass)) end
print(c(GREEN, "  ALL GREEN — " .. table.concat(parts, " · ")))
