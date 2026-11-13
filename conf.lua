BIN = "target/release/alm"

OUT = "target/run_out.txt"
ERR = "target/run_err.txt"
BERR = "target/build_err.txt"

CASES_DIR = "cases"
LOCK_DIR = "lock"
DIFF_DIR = "diff"

os.execute("mkdir -p target")

function read_file(path)
    local f = io.open(path, "r")
    if not f then return "" end
    local content = f:read("*a")
    f:close()
    return content
end

function copy_file(src, dst)
    local i = io.open(src, "rb")
    if not i then return false end
    local data = i:read("*a")
    i:close()
    local o = io.open(dst, "wb")
    if not o then return false end
    o:write(data)
    o:close()
    return true
end

function clean_artifacts()
    os.remove("libalm_out.ll")
    os.remove("libalm_out.so")
    os.remove("alm_out.ll")
    os.remove("alm_out")
    os.remove(".alm_trace.bin")
    os.remove(".alm_rt_trace.bin")
    os.remove(OUT)
    os.remove(ERR)
end

-- Execute a shell command and return (ok, timed_out, exit_code)
function exec_cmd(cmd)
    local ok, how, code = os.execute(cmd)
    if type(ok) == "number" then
        -- Lua 5.1 compatibility: os.execute returns raw waitpid status
        local status = (ok >= 256) and math.floor(ok / 256) or ok
        return status == 0, status == 124, status
    end
    local timed_out = (how == "exit" and code == 124)
    return ok == true, timed_out, code or -1
end

function split_lines(s)
    local t = {}
    if s == "" then return t end
    s = s:gsub("\r\n", "\n"):gsub("\r", "\n")
    if s:sub(-1) == "\n" then
        s = s:sub(1, -2)
    end
    for line in (s .. "\n"):gmatch("(.-)\n") do
        table.insert(t, line)
    end
    return t
end

-- Pin grammar and classification rules: see README.md.
-- Unknown EXPECT_* prefixes are recorded in pins.unknown so the runner
-- can surface typos instead of silently dropping pins.
function parse_pins(content)
    local pins = {
        expect = {},
        build_fail = {},
        panic = {},
        unknown = {},
        dup_meta = {}
    }
    for _, line in ipairs(split_lines(content)) do
        local prefix, rest = line:match("^%s*%-%-%s*(EXPECT[_A-Z]*):%s?(.*)$")
        if prefix then
            if prefix == "EXPECT" then
                table.insert(pins.expect, rest)
            elseif prefix == "EXPECT_BUILD_FAIL" then
                table.insert(pins.build_fail, rest)
            elseif prefix == "EXPECT_PANIC" then
                table.insert(pins.panic, rest)
            else
                table.insert(pins.unknown, prefix)
            end
        else
            local args = line:match("^%s*%-%-%s*ARGS:%s*(.+)$")
            if args then
                if pins.args then table.insert(pins.dup_meta, "ARGS") end
                pins.args = args
            end
            local flags = line:match("^%s*%-%-%s*FLAGS:%s*(.+)$")
            if flags then
                if pins.flags then table.insert(pins.dup_meta, "FLAGS") end
                pins.flags = flags
            end
        end
    end
    return pins
end

function scan_corpus()
    local corpus = {
        positive = {},
        negative = {},
        panic = {},
        bad = {},
        odd = {},
        pins = {}
    }
    local p = io.popen("ls '" .. CASES_DIR .. "' 2>/dev/null")
    for name in p:lines() do
        if name:sub(-4) == ".lua" then
            local pins = parse_pins(read_file(CASES_DIR .. "/" .. name))
            corpus.pins[name] = pins
            local is_neg, is_pan = #pins.build_fail > 0, #pins.panic > 0
            local why
            if is_neg and is_pan then
                why = "carries both EXPECT_BUILD_FAIL and EXPECT_PANIC pins"
            elseif is_neg and #pins.expect > 0 then
                why = "negative (EXPECT_BUILD_FAIL) but also carries EXPECT pins"
            elseif #pins.dup_meta > 0 then
                why = "duplicate metadata pin(s): " .. table.concat(pins.dup_meta, ", ")
            end
            if why then
                table.insert(corpus.bad, { name, why })
            elseif is_neg then
                table.insert(corpus.negative, name)
            elseif is_pan then
                table.insert(corpus.panic, name)
            else
                table.insert(corpus.positive, name)
            end
            if #pins.unknown > 0 then
                table.insert(corpus.odd, { name, table.concat(pins.unknown, ", ") })
            end
        end
    end
    p:close()
    return corpus
end

function run_case(src, args, with_backtrace, flags)
    clean_artifacts()
    local bt = with_backtrace and "RUST_BACKTRACE=1 " or ""
    local argstr = args and (" " .. args) or ""
    local flagstr = flags and (flags .. " ") or ""
    local cmd = string.format(
        "%stimeout 120 %s %s'%s'%s > %s 2> %s",
        bt, BIN, flagstr, src, argstr, OUT, ERR)
    return exec_cmd(cmd)
end

function compile_succeeded()
    return read_file(OUT):find("Success! Shared library written", 1, true) ~= nil
end

function compile_succeeded_exe()
    return read_file(OUT):find("Success! executable written", 1, true) ~= nil
end

function run_case_exe(src, flags)
    clean_artifacts()
    local clean_flags = flags and flags:gsub("%-%-exe%s*", "") or ""
    local flagstr = (clean_flags ~= "") and (clean_flags .. " ") or ""
    local cmd = string.format(
        "timeout 120 %s %s--exe '%s' > %s 2> %s",
        BIN, flagstr, src, OUT, ERR)
    return exec_cmd(cmd)
end

function run_exe_with(argstr, with_backtrace)
    os.remove(OUT)
    os.remove(ERR)
    local bt = with_backtrace and "RUST_BACKTRACE=1 " or ""
    local cmd = string.format(
        "%stimeout 120 ./alm_out%s > %s 2> %s",
        bt, argstr or "", OUT, ERR)
    return exec_cmd(cmd)
end

function compiler_panicked()
    local s = read_file(ERR)
    return s:find("panicked at", 1, true) ~= nil
        or s:find("thread 'main' panicked", 1, true) ~= nil
end

-- Strip the compiler's own success banner from stdout so only the script's
-- stdout lines remain (error reports go to stderr and never touch EXPECT pins).
function script_stdout()
    local lines = {}
    for _, line in ipairs(split_lines(read_file(OUT))) do
        if not line:find("Success! Shared library written", 1, true)
            and not line:find("Success! executable written", 1, true) then
            table.insert(lines, line)
        end
    end
    return table.concat(lines, "\n")
end

function first_error(path, timed_out)
    if timed_out then
        return "TIMEOUT (exceeded 120s limit)"
    end
    local s = read_file(path)
    local msg = s:match("panicked at.-\n%s*([^\n]+)")
    if not msg then
        for _, line in ipairs(split_lines(s)) do
            if line:find("Error", 1, true) then
                msg = line
                break
            end
        end
    end
    return msg
        or s:match("([^\r\n]+)")
        or "(no error output)"
end

-- The lock oracle normalizes the DIFile directory out of the compared
-- IR: the compiler truthfully embeds the source's absolute directory
-- (backend.rs DebugMeta), which would make every lock machine-absolute
-- and every differently-named checkout read as 100% drift. Both sides
-- of the comparison are normalized (the freshly captured IR here, the
-- lock on disk at diff time in run.lua), so absolute-path locks from
-- before the normalization keep passing, and reset.lua writes portable
-- ones. The filename stays: it names the case and does not vary.
function normalize_ir(code)
    return (code:gsub('(!DIFile%([^\n]-directory: ")([^"]*)(")', "%1<CASES_DIR>%3"))
end

function captured_ir()
    local code = read_file("libalm_out.ll")
    if code ~= "" then return normalize_ir(code) end
    return nil
end

-- ffi.load stays --exe-only (the M4 gate: a .so may ride the callee
-- seam, but never loads), so cases that load compile the standalone
-- lane: their IR lands in alm_out.ll, and the battery drives them
-- through the exe path. Callee-seam-only positives (cdef/new/sizeof,
-- no load) ride the lib lane like everything else.
function flags_exe(flags)
    return flags ~= nil and flags:find("--exe", 1, true) ~= nil
end

function captured_ir_exe()
    local code = read_file("alm_out.ll")
    if code ~= "" then return normalize_ir(code) end
    return nil
end
