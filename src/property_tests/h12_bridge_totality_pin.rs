
#[test]
fn h12_bridge_totality_pin() {
    let source = "\
local t = {}
t[\"k\"] = 7
local function Get(s) return t[s] end
local function Twice() return Get(\"k\") + Get(\"k\") end
local base = 1
local function Add1(x) return x + base end
local function Drive(f, n) return f(n) end
print(Twice())
print(Twice())
print(Drive(Add1, 5))
local function Inner(x) return x + 1 end
local function Outer(x)
    while x < 10 do
        x = Inner(x)
    end
    return x
end
while base < 3 do
    base = base + 1
    local q = Twice()
    local r = Outer(base)
end
local rec = { fn = Add1 }
local z = rec.fn(2)
local function Maker(k)
    local function GetK() return k end
    return GetK
end
local gk = Maker(7)
local w = gk()
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "H12 pin source must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    for rec in shape.instances.values() {
        for (&call, kids) in &rec.children {
            assert!(
                !kids.is_empty(),
                "the children map never carries an empty set (call {call:?})"
            );
            for &kid in kids {
                assert!(
                    shape.instances.contains_key(&kid),
                    "children names instance {kid:?} with no record"
                );
                assert_eq!(
                    shape.instances[&kid].call,
                    Some(call),
                    "a child minted under call {call:?} names a different call"
                );
            }
        }
    }

    let mut reached = std::collections::BTreeSet::from([crate::shape::InstanceId::ROOT]);
    for rec in shape.instances.values() {
        reached.extend(rec.children.values().flatten().copied());
    }
    for &k in shape.instances.keys() {
        assert!(
            reached.contains(&k),
            "instance {k:?} is unreachable from ROOT on a clean compile"
        );
    }
    let only_child = |owner: crate::shape::InstanceId, call: NodeId| {
        *shape.instances[&owner].children[&call]
            .iter()
            .next()
            .unwrap()
    };

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert!(root.call.is_none() && root.fn_node.is_none() && root.ret.is_none());
    assert_eq!(root.frame_base, 0);
    assert_eq!(root.break_base, 0);
    assert!(root.params.is_empty() && root.body.is_empty());
    assert!(
        !root.creation_envs.is_empty(),
        "the creation site bridges to ROOT"
    );

    let rec_fn_call = match &ast[14] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("the last statement binds the field dispatch"),
    };
    assert!(
        root.closure_calls.contains(&rec_fn_call),
        "the field dispatch's kind rides ROOT's record"
    );
    assert!(
        root.children.contains_key(&rec_fn_call),
        "a top-level self-pair's events mint under ROOT (children is the \
         owner's map)"
    );
    let k_disp = *root.children[&rec_fn_call].iter().next().unwrap();
    let add1_lit = match &ast[5] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("Add1's literal is the sixth statement"),
    };
    let disp_rec = &shape.instances[&k_disp];
    assert_eq!(
        disp_rec.fn_node,
        Some(add1_lit),
        "the record's field pins Add1"
    );
    let face = disp_rec
        .env_face
        .as_ref()
        .expect("the call-through records its env face");
    assert!(
        face.is_empty(),
        "a top-level fn literal captures nothing — the flat module \
         resolves top-level names late: {face:?}"
    );

    let maker_call = match &ast[16] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("gk binds Maker's call"),
    };
    let k_maker = only_child(crate::shape::InstanceId::ROOT, maker_call);
    let getk_value = {
        let maker_body = match &ast[15] {
            Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
                Expr::Function { body, .. } => body,
                _ => panic!("Maker binds a fn literal"),
            },
            _ => panic!("Maker's declaration is the sixteenth statement"),
        };
        match &maker_body[1] {
            Stmt::Return {
                value: Some(value @ Expr::Identifier { .. }),
                ..
            } => value.id(),
            _ => panic!("Maker returns the GetK value read"),
        }
    };
    assert_eq!(
        shape.instances[&k_maker].creation_envs[&getk_value],
        match &ast[15] {
            Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
                Expr::Function { body, .. } => match &body[0] {
                    Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
                    _ => panic!("GetK's literal is Maker's first body statement"),
                },
                _ => panic!(),
            },
            _ => panic!(),
        },
        "the owner record's creation_envs names the escaping literal — \
         the creation keys against the ENCLOSING expansion (the site law)"
    );
    let gk_call = match &ast[17] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("w binds the dispatch"),
    };
    let k_gk = only_child(crate::shape::InstanceId::ROOT, gk_call);
    let gk_rec = &shape.instances[&k_gk];
    let face = gk_rec
        .env_face
        .as_ref()
        .expect("the factory dispatch records its env face");
    assert_eq!(
        face.len(),
        1,
        "R3: GetK's capture rides the instance — one cell"
    );
    assert_eq!(
        face.get("k"),
        Some(&crate::shape::Ty::Int),
        "the resolved face carries k's cell type"
    );

    let fn_body = |stmt: usize| match &ast[stmt] {
        Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
            Expr::Function { body, .. } => body,
            _ => panic!("the local binds a fn literal"),
        },
        _ => panic!("the statement declares a fn"),
    };
    let outer_call_in_loop = match &ast[12] {
        Stmt::While { body, .. } => match &body[2] {
            Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
            _ => panic!("the loop's third statement binds Outer's call"),
        },
        _ => panic!("the final statement is the top-level while"),
    };
    let k_outer_loop = only_child(crate::shape::InstanceId::ROOT, outer_call_in_loop);
    assert_eq!(
        shape.instances[&k_outer_loop].break_base, 1,
        "a call inside one top-level loop mints at absolute base 1"
    );
    let inner_call = match &fn_body(11)[0] {
        Stmt::While { body, .. } => match &body[0] {
            Stmt::Assignment { expr, .. } => match expr {
                Expr::Call { .. } => expr.id(),
                _ => panic!("Outer's loop assigns Inner's call"),
            },
            _ => panic!("Outer's loop opens with the assignment"),
        },
        _ => panic!("Outer's body opens with the while"),
    };
    let k_inner_loop = only_child(k_outer_loop, inner_call);
    assert_eq!(
        shape.instances[&k_inner_loop].break_base, 2,
        "R1: enclosing base 1 + Outer-relative depth 1 — the ABSOLUTE \
         base, the lowerer's break_ctxs.len() twin"
    );
    assert_eq!(
        shape.instances[&k_inner_loop].frame_base, 3,
        "frames: the top loop's scope, Outer's expansion scope, and \
         Outer's while scope below"
    );

    assert!(
        compile_exe(source).is_ok(),
        "the lowerer lowers this shape clean — the cursor's picks are \
         event-exact"
    );

    let loop_twice_call = match &ast[12] {
        Stmt::While { body, .. } => match &body[1] {
            Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
            _ => panic!("the loop's second statement binds Twice's call"),
        },
        _ => panic!("ast[12] is the top-level while"),
    };
    let k_twice = only_child(crate::shape::InstanceId::ROOT, loop_twice_call);
    let twice_rec = &shape.instances[&k_twice];
    assert_eq!(
        twice_rec.children.len(),
        2,
        "Twice() expands two distinct call sites for Get(\"k\") + Get(\"k\")"
    );
}
