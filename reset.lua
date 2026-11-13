#!/usr/bin/env lua
local script_path = arg[0] or debug.getinfo(1, "S").source:match("^@(.+)$") or "."
local conf_dir = script_path:match("^(.+)[/\\]") or "."
dofile(conf_dir .. "/conf.lua")

local function usage()
    print([[
Rewrites every buildable case's IR baseline in ]] .. LOCK_DIR .. [[/.

Usage: lua reset.lua]])
end

local ARGS = { ... }
if ARGS[1] and ARGS[1] ~= "run" then
    usage()
    io.stderr:write("\nunknown argument: " .. ARGS[1] .. "\n")
    os.exit(1)
end

do
    local ok = exec_cmd("cargo build --release --quiet > /dev/null 2> " .. BERR)
    if not ok then
        print("\27[31mcargo build failed — refusing to lock:\27[0m\n" .. read_file(BERR))
        os.exit(1)
    end
end

local corpus = scan_corpus()
if #corpus.bad > 0 then
    for _, b in ipairs(corpus.bad) do
        print(string.format("\27[31m✗\27[0m %s — classification conflict, nothing locked: %s", b[1], b[2]))
    end
    os.exit(1)
end

local lockables = {}
for _, n in ipairs(corpus.positive) do table.insert(lockables, n) end
for _, n in ipairs(corpus.panic) do table.insert(lockables, n) end

-- Stage locks in a temporary directory so a single compile failure never leaves
-- LOCK_DIR in a half-overwritten state.
local STAGING_DIR = "target/lock_staging"
os.execute("rm -rf '" .. STAGING_DIR .. "'")
os.execute("mkdir -p '" .. STAGING_DIR .. "'")

local failed = {}
for _, filename in ipairs(lockables) do
    local pins = corpus.pins[filename]
    local code, timed_out
    if flags_exe(pins.flags) then
        -- The ffi surface's lane: compile standalone, lock alm_out.ll.
        local _, to = run_case_exe(CASES_DIR .. "/" .. filename, pins.flags)
        timed_out = to
        code = compile_succeeded_exe() and captured_ir_exe() or nil
    else
        local _, to = run_case(CASES_DIR .. "/" .. filename, nil, false, pins.flags)
        timed_out = to
        code = compile_succeeded() and captured_ir() or nil
    end
    if not code then
        print(string.format("\27[31m✗\27[0m %s — compile failed (or emitted .ll empty): %s",
            filename, first_error(ERR, timed_out)))
        table.insert(failed, filename)
    else
        local dest = STAGING_DIR .. "/" .. filename:gsub("%.lua$", "") .. ".ll"
        local f = io.open(dest, "w")
        f:write(code)
        f:close()
    end
end

if #failed > 0 then
    os.execute("rm -rf '" .. STAGING_DIR .. "'")
    print(string.format("\27[31mLOCKDOWN ABORTED\27[0m — %d/%d failed: %s",
        #failed, #lockables, table.concat(failed, ", ")))
    print("Existing lock files in " .. LOCK_DIR .. "/ were left untouched.")
    os.exit(1)
end

os.execute("mkdir -p '" .. LOCK_DIR .. "'")
for _, filename in ipairs(lockables) do
    local ll_name = filename:gsub("%.lua$", "") .. ".ll"
    local src = STAGING_DIR .. "/" .. ll_name
    local dest = LOCK_DIR .. "/" .. ll_name
    copy_file(src, dest)
    print(string.format("\27[32m✓\27[0m locked %s", dest))
end
os.execute("rm -rf '" .. STAGING_DIR .. "'")

do
    local expected = {}
    for _, n in ipairs(lockables) do expected[n:gsub("%.lua$", "") .. ".ll"] = true end
    local p = io.popen("ls '" .. LOCK_DIR .. "' 2>/dev/null")
    for line in p:lines() do
        if not expected[line] then
            os.remove(LOCK_DIR .. "/" .. line)
            print("removed orphan lock: " .. LOCK_DIR .. "/" .. line)
        end
    end
    p:close()
end

clean_artifacts()

io.write(string.format("== milestone locked: %d tests -> %s/ ==\n", #lockables, LOCK_DIR))
do
    local p = io.popen("git rev-parse --short HEAD 2>/dev/null")
    local h = p:read("*l") or "?"
    p:close()
    io.write(string.format("   at commit %s, %s\n", h, os.date("locked %Y-%m-%d %H:%M:%S")))
end
