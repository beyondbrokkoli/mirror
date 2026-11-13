impl<'a> IrLowerer<'a> {

    fn lower_expr(&mut self, expr: &Expr, target: Option<RegId>) -> Result<TypedReg, LowerError> {
        let reg = target.unwrap_or_else(|| self.next_reg());

        match expr {
            Expr::Integer { value: val, .. } => {
                self.emit(Instruction::LoadInt {
                    target: Reg::new(reg),
                    val: *val,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            Expr::Float { value: val, .. } => {
                self.emit(Instruction::LoadFloat {
                    target: Reg::new(reg),
                    val: *val,
                });
                Ok(TypedReg {
                    reg: AnyReg::Float(Reg::new(reg)),
                    ty: Ty::Flt,
                })
            }
            Expr::Boolean { value: val, .. } => {
                self.emit(Instruction::LoadBool {
                    target: Reg::new(reg),
                    val: *val,
                });
                Ok(TypedReg {
                    reg: AnyReg::Bool(Reg::new(reg)),
                    ty: Ty::Bool,
                })
            }
            Expr::String { value: val, .. } => {
                self.emit(Instruction::LoadString {
                    target: Reg::new(reg),
                    val: val.clone(),
                });
                Ok(TypedReg {
                    reg: AnyReg::Str(Reg::new(reg)),
                    ty: Ty::Str,
                })
            }
            Expr::Nil { .. } => {
                unreachable!("nil outside a table release")
            }
            Expr::SysAllocCount { .. } => {
                self.emit(Instruction::SysAllocCount {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            Expr::SysExecCount { .. } => {
                self.emit(Instruction::SysExecCount {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            Expr::FfiModule { .. } => {
                self.emit(Instruction::LoadNull {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::Ffi,
                })
            }
            Expr::AsyncModule { .. } => {
                self.emit(Instruction::LoadNull {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::Async,
                })
            }
            Expr::Function { .. } => {
                self.emit(Instruction::LoadNull {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::Unknown(0),
                })
            }
            Expr::Call { callee, args, .. } => {
                let call_ptr = expr.id();
                let owner = self
                    .instance_chain
                    .first()
                    .copied()
                    .unwrap_or(InstanceId::ROOT);
                let kids = self.shape.instances[&owner].children.get(&call_ptr);
                if let Expr::Field { .. } = callee.as_ref() && kids.is_none() {
                    return self.lower_ffi_call(expr, args, reg);
                }
                if kids.is_none()
                    && let Some(verb) = self.shape.builtin_verbs.get(&call_ptr).cloned()
                {
                    return self.lower_builtin_call(verb, args, reg);
                }
                let kids = kids.expect(
                    "the checker resolved this call as an expansion — children is the \
                     record view of the resolved call (7b: the fact-site guard's \
                     successor)",
                );
                let first_kid = *kids.iter().next().unwrap();
                let fn_ptr = self.shape.instances[&first_kid]
                    .fn_node
                    .expect("a minted instance names its callee (7b)");
                let call_env: Option<Reg<Ptr>> = match callee.as_ref() {
                    Expr::Identifier { name: n, .. } => match self.read_var(n) {
                        Ok(local) if matches!(local.ty, Ty::Closure(_)) => Some(ptr_of(local.reg)?),
                        _ => None,
                    },
                    Expr::Field { obj, name, .. }
                        if self.shape.instances[&owner].closure_calls.contains(&call_ptr) =>
                    {
                        let t_any = self.lower_expr(obj, None)?.reg;
                        let t_reg = ptr_of(t_any)?;
                        let k = self.next_reg();
                        self.emit(Instruction::StrKeyConst {
                            target: Reg::new(k),
                            val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
                        });
                        let cell = self.next_reg();
                        self.emit(Instruction::TableGet(<Ptr as CellRepr>::get(
                            Reg::new(cell),
                            t_reg,
                            TableKey::Str(Reg::new(k)),
                        )));
                        Some(Reg::new(cell))
                    }
                    // C: only a closure value or a blessed field call
                    // carries an env; every other callee shape is bare
                    _ => None,
                };
                if self
                    .instance_chain
                    .iter()
                    .any(|i| self.shape.instances[i].fn_node == Some(fn_ptr))
                {
                    panic!(
                        "recursive inlining — the checker refuses it before \
                         any compile reaches the lowerer; the call graph is \
                         a DAG on every compile that gets here"
                    );
                }
                let def = self.shape.fn_defs.get(&fn_ptr).cloned().expect(
                    "a call site names a function with no body — fn_defs \
                     feeds every resolver; the checker records every \
                     accepted call's literal",
                );
                let call_ret = self.shape.instances[&first_kid]
                    .ret
                    .clone()
                    .expect(
                        "a call site with no resolved return — the mint's \
                         ret fills at the expansion's exit, before the \
                         lowerer ever sees the kid",
                    );
                let scope_key = self.shape.fn_scope_keys.get(&fn_ptr).copied();
                let body = def.body();

                let mut arg_regs = Vec::with_capacity(args.len());
                for a in args {
                    let tr = self.lower_expr(a, None)?;
                    let layout = self.lookup_layout_for(a)?;
                    arg_regs.push((tr, layout));
                }

                let k = *self
                    .event_cursor
                    .entry((owner, call_ptr))
                    .or_insert(0);
                let kid = *kids.iter().nth(k).expect(
                    "the cursor outran its pair's minted events — \
                     one minter, pre-order consumption, and visits to \
                     one (owner, expr) never interleave: the cursor \
                     cannot run past its mints (7b)",
                );
                *self
                    .event_cursor
                    .get_mut(&(owner, call_ptr))
                    .unwrap() += 1;
                self.instance_chain.push(kid);
                self.return_ctxs.push(ReturnCtx {
                    kind: ReturnKind::Inline {
                        ty: call_ret.clone(),
                    },
                    exit: None,
                    args: Vec::new(),
                });
                self.in_block(|an| {
                    for (p, (tr, layout)) in def.params.iter().zip(arg_regs) {
                        an.declare_var(p.clone(), tr.reg, tr.ty, layout);
                    }
                    if let Some(env) = call_env {
                        an.enter_env_ctx(env, fn_ptr)?;
                    }
                    for s in body {
                        an.lower_stmt(s)?;
                    }
                    match scope_key {
                        Some(key) => an.emit_scope_frees(key),
                        None => Ok(()),
                    }
                })?;
                let result = self.finalize_return_ctx()?.expect(
                    "an inline call produced no value — the minted body \
                     carries a valued return (the checker's R15 gate)",
                );
                if let Some(classes) = self.shape.call_tags.get(&expr.id()).cloned() {
                    let exit = self.current_block;
                    let edge_blocks: Vec<BlockId> = self.blocks[exit]
                        .instrs
                        .iter()
                        .find_map(|i| match i {
                            Instruction::Phi(p) => Some(p.arg_blocks().to_vec()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    if edge_blocks.len() < classes.len() {
                        panic!(
                            "a tagged call's return edges went missing — the \
                             phi carries one edge per tag class"
                        );
                    }
                    let mut tag_args: Vec<(BlockId, AnyReg)> = Vec::new();
                    for (i, &b) in edge_blocks.iter().enumerate() {
                        let class = classes.get(i).copied().unwrap_or(false);
                        let r = self.next_reg();
                        let block = &mut self.blocks[b];
                        block.locs.push(self.cur_loc);
                        block.instrs.push(Instruction::LoadBool {
                            target: Reg::new(r),
                            val: class,
                        });
                        tag_args.push((b, AnyReg::Bool(Reg::new(r))));
                    }
                    let tag_reg = self.next_reg();
                    let phi = phi_of(RegKind::Bool, tag_reg, tag_args)?;
                    self.emit(Instruction::Phi(phi));
                    self.call_tag_regs.insert(expr.id(), Reg::new(tag_reg));
                }
                self.instance_chain.pop();
                if call_env.is_some() {
                    self.env_ctxs.pop();
                }
                Ok(TypedReg {
                    reg: result,
                    ty: call_ret,
                })
            }
            Expr::TableCtor { entries, .. } => {
                let elem = self.shape.elem_of(expr);
                let ctor_site = self.shape.sites.get(&expr.id()).copied();

                let has_closure_fields = entries.iter().any(|(k, _)| match k {
                    CtorKey::Str(n) => ctor_site
                        .is_some_and(|s| self.shape.closure_fields.contains_key(&(s, n.clone()))),
                    _ => false,
                });

                if matches!(elem, crate::shape::Ty::Unknown(_)) {
                    signal!(trace::TRACE_TABLE_NEW_UNDECIDED);
                }

                let mut has_inline_tables = false;
                for (_, e) in entries {
                    if matches!(e, Expr::TableCtor { .. }) {
                        has_inline_tables = true;
                    }
                }

                let is_stored_parent =
                    ctor_site.is_some_and(|s| self.shape.stored_ctor_parents.contains(&s));

                let contains_tables = matches!(elem, crate::shape::Ty::Tbl(_))
                    && (has_inline_tables || is_stored_parent);

                let mut elem_regs = Vec::with_capacity(entries.len());
                for (key, e) in entries {
                    let TypedReg { reg: r, .. } = self.lower_expr(e, None)?;
                    let is_closure_field = match key {
                        CtorKey::Str(n) => ctor_site.is_some_and(|s| {
                            self.shape.closure_fields.contains_key(&(s, n.clone()))
                        }),
                        _ => false,
                    };
                    let r = if matches!(elem, Ty::Any) && !is_closure_field {
                        self.pack_any(r)?
                    } else {
                        r
                    };
                    elem_regs.push(r);
                }

                if let Some(pairs) = self.shape.ctor_ghost_keeps.get(&expr.id()) {
                    for &(ei, g) in pairs {
                        if let Some(&r) = elem_regs.get(ei)
                            && let Ok(ptr) = ptr_of(r)
                        {
                            self.ghost_keeps.insert(g, ptr);
                        }
                    }
                }

                let mode_bit = match self.lookup_layout_for(expr)? {
                    crate::shape::LayoutVerdict::Sparse => 0x01,
                    _ => 0x00,
                };
                let flags = mode_bit | if contains_tables { 0x80 } else { 0x00 };

                let tbl = Reg::<Ptr>::new(reg);
                let new_elem = if has_closure_fields {
                    Ty::Str
                } else {
                    elem.clone()
                };
                self.emit(Instruction::TableNew {
                    target: tbl,
                    elem: new_elem,
                    flags,
                });
                if let Some(&site) = self.shape.sites.get(&expr.id()) {
                    self.handles.site_regs.insert(site, (tbl, self.ctrl_depth));
                }
                for ((key, _), v_reg) in entries.iter().zip(elem_regs) {
                    match key {
                        CtorKey::Const(slot) => {
                            let i_reg = self.next_reg();
                            self.emit(Instruction::LoadInt {
                                target: Reg::new(i_reg),
                                val: *slot,
                            });
                            self.table_set(tbl, TableKey::Int(Reg::new(i_reg)), v_reg, &elem)?;
                        }
                        CtorKey::Str(name) => {
                            let k = self.next_reg();
                            self.emit(Instruction::StrKeyConst {
                                target: Reg::new(k),
                                val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
                            });
                            let store_face = match ctor_site
                                .and_then(|s| self.shape.closure_fields.get(&(s, name.clone())))
                            {
                                Some(&ptr) => Ty::Closure(ptr),
                                None => elem.clone(),
                            };
                            self.table_set(tbl, TableKey::Str(Reg::new(k)), v_reg, &store_face)?;
                        }
                        CtorKey::Expr(ke) => {
                            let key = self.lower_index_key(ke)?;
                            self.table_set(tbl, key, v_reg, &elem)?;
                        }
                    }
                }
                Ok(TypedReg {
                    reg: AnyReg::Ptr(tbl),
                    ty: Ty::Tbl(Box::new(elem)),
                })
            }
            Expr::Index { obj, key, .. } => {
                let t_any = self.lower_expr(obj, None)?.reg;
                let key = self.lower_index_key(key)?;
                let target_ty = self.elem_face_of(obj)?;
                let t_reg = ptr_of(t_any)?;
                let out = self.table_get(reg, t_reg, key, &target_ty);
                Ok(TypedReg {
                    reg: out,
                    ty: target_ty,
                })
            }
            Expr::Field { obj, name, .. } => {
                let get_reg = if target.is_some() {
                    reg
                } else {
                    self.next_reg()
                };
                let TypedReg { reg: t_any, .. } = self.lower_expr(obj, None)?;
                let k = self.next_reg();
                self.emit(Instruction::StrKeyConst {
                    target: Reg::new(k),
                    val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
                });
                let target_ty = self.elem_face_of(obj)?;
                let t_reg = ptr_of(t_any)?;
                let out = self.table_get(get_reg, t_reg, TableKey::Str(Reg::new(k)), &target_ty);
                Ok(TypedReg {
                    reg: out,
                    ty: target_ty,
                })
            }
            Expr::Identifier { name, .. } => {
                let owner = self
                    .instance_chain
                    .first()
                    .copied()
                    .unwrap_or(InstanceId::ROOT);
                if let Some(&ptr) = self.shape.instances[&owner]
                    .creation_envs
                    .get(&expr.id())
                {
                    let reg = self.mint_env(ptr)?;
                    return Ok(TypedReg {
                        reg: AnyReg::Ptr(reg),
                        ty: Ty::Closure(ptr),
                    });
                }
                let local = self.read_var(name)?;
                if target.is_some() && reg != local.reg.id() {
                    let moved = self.emit_move_into(reg, local.reg);
                    Ok(TypedReg {
                        reg: moved,
                        ty: local.ty,
                    })
                } else {
                    Ok(TypedReg {
                        reg: local.reg,
                        ty: local.ty,
                    })
                }
            }
            Expr::BinaryOp {
                op: op @ (BinOp::And | BinOp::Or),
                left,
                right,
                ..
            } => {
                let l_reg = bool_of(self.lower_expr(left, None)?.reg)?;
                self.emit_arm_temp_frees(left);
                let then_block = self.new_block();
                let else_block = self.new_block();
                let join_block = self.new_block();
                self.terminate(Terminator::Branch {
                    cond: l_reg,
                    true_block: then_block,
                    false_block: else_block,
                });
                let and_op = matches!(op, BinOp::And);
                let (value_block, const_block, short_val) = if and_op {
                    (then_block, else_block, false)
                } else {
                    (else_block, then_block, true)
                };

                self.current_block = value_block;
                let v_reg = self.at_depth(|an| an.lower_expr(right, None))?.reg;
                let value_end = self.current_block;
                self.emit_arm_temp_frees(right);
                self.terminate(Terminator::Jump(join_block));

                self.current_block = const_block;
                let c_reg = self.next_reg();
                self.emit(Instruction::LoadBool {
                    target: Reg::new(c_reg),
                    val: short_val,
                });
                let const_end = const_block;
                self.terminate(Terminator::Jump(join_block));

                self.current_block = join_block;
                let (t_end, t_reg, e_end, e_reg) = if and_op {
                    (value_end, v_reg, const_end, AnyReg::Bool(Reg::new(c_reg)))
                } else {
                    (const_end, AnyReg::Bool(Reg::new(c_reg)), value_end, v_reg)
                };
                let phi = phi_of(RegKind::Bool, reg, vec![(t_end, t_reg), (e_end, e_reg)])?;
                self.emit(Instruction::Phi(phi));
                Ok(TypedReg {
                    reg: AnyReg::Bool(Reg::new(reg)),
                    ty: Ty::Bool,
                })
            }
            Expr::BinaryOp {
                op, left, right, ..
            } => {
                let TypedReg {
                    reg: l_any,
                    ty: l_ty,
                } = self.lower_expr(left, None)?;
                let TypedReg {
                    reg: r_any,
                    ty: r_ty,
                } = self.lower_expr(right, None)?;
                match op {
                    BinOp::Concat => {
                        // Literal pairs fold here before any runtime concat
                        // is minted, so "a" .. "b" .. x costs one
                        // alm_str_concat, not two.
                        if let (Expr::String { value: lv, .. }, Expr::String { value: rv, .. }) =
                            (left.as_ref(), right.as_ref())
                        {
                            let joined = format!("{lv}{rv}");
                            self.emit(Instruction::LoadString {
                                target: Reg::new(reg),
                                val: joined,
                            });
                            return Ok(TypedReg {
                                reg: AnyReg::Str(Reg::new(reg)),
                                ty: Ty::Str,
                            });
                        }
                        let l = str_lane(l_any)?;
                        let r = str_lane(r_any)?;
                        self.emit(Instruction::StrConcat {
                            target: Reg::new(reg),
                            l,
                            r,
                        });
                        return Ok(TypedReg {
                            reg: AnyReg::Str(Reg::new(reg)),
                            ty: Ty::Str,
                        });
                    }
                    BinOp::Add => match num_pair(l_any, r_any)? {
                        NumPair::II(a, b) => self.num3(Instruction::Add, reg, a, b),
                        NumPair::FF(a, b) => self.num3(Instruction::Add, reg, a, b),
                    },
                    BinOp::Sub => match num_pair(l_any, r_any)? {
                        NumPair::II(a, b) => self.num3(Instruction::Sub, reg, a, b),
                        NumPair::FF(a, b) => self.num3(Instruction::Sub, reg, a, b),
                    },
                    BinOp::Mul => match num_pair(l_any, r_any)? {
                        NumPair::II(a, b) => self.num3(Instruction::Mul, reg, a, b),
                        NumPair::FF(a, b) => self.num3(Instruction::Mul, reg, a, b),
                    },
                    BinOp::Div => {
                        let l_reg = self.promote_to_float(l_any, &l_ty)?;
                        let r_reg = self.promote_to_float(r_any, &r_ty)?;
                        self.emit(Instruction::Div {
                            target: Reg::new(reg),
                            left: l_reg,
                            right: r_reg,
                        });
                    }
                    BinOp::IntDiv => {
                        let rhs_const = const_of(right);
                        match num_pair(l_any, r_any)? {
                            NumPair::II(a, b) => {
                                self.num3r(Instruction::IntDiv, reg, a, b, rhs_const)
                            }
                            NumPair::FF(a, b) => {
                                self.num3r(Instruction::IntDiv, reg, a, b, rhs_const)
                            }
                        }
                    }
                    BinOp::Mod => {
                        let rhs_const = const_of(right);
                        match num_pair(l_any, r_any)? {
                            NumPair::II(a, b) => self.num3r(Instruction::Mod, reg, a, b, rhs_const),
                            NumPair::FF(a, b) => self.num3r(Instruction::Mod, reg, a, b, rhs_const),
                        }
                    }
                    BinOp::LessThan => match ord_pair(l_any, r_any)? {
                        OrdPair::II(a, b) => self.cmp3(Instruction::Less, reg, a, b),
                        OrdPair::FF(a, b) => self.cmp3(Instruction::Less, reg, a, b),
                        OrdPair::BB(a, b) => self.cmp3(Instruction::Less, reg, a, b),
                        OrdPair::SS(a, b) => self.cmp3(Instruction::Less, reg, a, b),
                        OrdPair::PP(a, b) => self.cmp3(Instruction::Less, reg, a, b),
                        OrdPair::AA(..) => panic!(
                            "an ordering comparison on Any boundary cells — \
                             the checker rejects ordering through the Any \
                             boundary before the lowerer runs"
                        ),
                    },
                    BinOp::GreaterThan => match ord_pair(l_any, r_any)? {
                        OrdPair::II(a, b) => self.cmp3(Instruction::Less, reg, b, a),
                        OrdPair::FF(a, b) => self.cmp3(Instruction::Less, reg, b, a),
                        OrdPair::BB(a, b) => self.cmp3(Instruction::Less, reg, b, a),
                        OrdPair::SS(a, b) => self.cmp3(Instruction::Less, reg, b, a),
                        OrdPair::PP(a, b) => self.cmp3(Instruction::Less, reg, b, a),
                        OrdPair::AA(..) => panic!(
                            "an ordering comparison on Any boundary cells — \
                             the checker rejects ordering through the Any \
                             boundary before the lowerer runs"
                        ),
                    },
                    BinOp::LessEq => match ord_pair(l_any, r_any)? {
                        OrdPair::II(a, b) => self.cmp3(Instruction::Leq, reg, a, b),
                        OrdPair::FF(a, b) => self.cmp3(Instruction::Leq, reg, a, b),
                        OrdPair::BB(a, b) => self.cmp3(Instruction::Leq, reg, a, b),
                        OrdPair::SS(a, b) => self.cmp3(Instruction::Leq, reg, a, b),
                        OrdPair::PP(a, b) => self.cmp3(Instruction::Leq, reg, a, b),
                        OrdPair::AA(..) => panic!(
                            "an ordering comparison on Any boundary cells — \
                             the checker rejects ordering through the Any \
                             boundary before the lowerer runs"
                        ),
                    },
                    BinOp::GreaterEq => match ord_pair(l_any, r_any)? {
                        OrdPair::II(a, b) => self.cmp3(Instruction::Geq, reg, a, b),
                        OrdPair::FF(a, b) => self.cmp3(Instruction::Geq, reg, a, b),
                        OrdPair::BB(a, b) => self.cmp3(Instruction::Geq, reg, a, b),
                        OrdPair::SS(a, b) => self.cmp3(Instruction::Geq, reg, a, b),
                        OrdPair::PP(a, b) => self.cmp3(Instruction::Geq, reg, a, b),
                        OrdPair::AA(..) => panic!(
                            "an ordering comparison on Any boundary cells — \
                             the checker rejects ordering through the Any \
                             boundary before the lowerer runs"
                        ),
                    },
                    BinOp::Equal => match ord_pair(l_any, r_any)? {
                        OrdPair::II(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                        OrdPair::FF(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                        OrdPair::BB(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                        OrdPair::SS(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                        OrdPair::PP(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                        OrdPair::AA(a, b) => self.cmp3(Instruction::Eq, reg, a, b),
                    },
                    BinOp::NotEqual => {
                        let e = self.next_reg();
                        match ord_pair(l_any, r_any)? {
                            OrdPair::II(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                            OrdPair::FF(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                            OrdPair::BB(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                            OrdPair::SS(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                            OrdPair::PP(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                            OrdPair::AA(a, b) => self.cmp3(Instruction::Eq, e, a, b),
                        }
                        self.emit(Instruction::Not {
                            target: Reg::new(reg),
                            source: Reg::new(e),
                        });
                    }
                    BinOp::And | BinOp::Or => unreachable!(),
                }
                let ty = match op {
                    BinOp::LessThan
                    | BinOp::GreaterThan
                    | BinOp::LessEq
                    | BinOp::GreaterEq
                    | BinOp::Equal
                    | BinOp::NotEqual => Ty::Bool,
                    BinOp::Div => Ty::Flt,
                    _ => {
                        if matches!(l_ty, Ty::Flt) || matches!(r_ty, Ty::Flt) {
                            Ty::Flt
                        } else {
                            Ty::Int
                        }
                    }
                };
                Ok(TypedReg {
                    reg: repr_of(&ty).reg(reg),
                    ty,
                })
            }
            Expr::UnaryOp { op, expr, .. } => match op {
                UnOp::Neg => {
                    let TypedReg {
                        reg: x_any,
                        ty: x_ty,
                    } = self.lower_expr(expr, None)?;
                    let out = match num_single(x_any)? {
                        NumSingle::I(x) => {
                            self.neg1(reg, x);
                            AnyReg::Int(Reg::new(reg))
                        }
                        NumSingle::F(x) => {
                            self.neg1(reg, x);
                            AnyReg::Float(Reg::new(reg))
                        }
                    };
                    Ok(TypedReg { reg: out, ty: x_ty })
                }
                UnOp::Not => {
                    let x_reg = bool_of(self.lower_expr(expr, None)?.reg)?;
                    self.emit(Instruction::Not {
                        target: Reg::new(reg),
                        source: x_reg,
                    });
                    Ok(TypedReg {
                        reg: AnyReg::Bool(Reg::new(reg)),
                        ty: Ty::Bool,
                    })
                }
                UnOp::Len => {
                    if let Expr::String { value: s, .. } = expr.as_ref() {
                        self.emit(Instruction::LoadInt {
                            target: Reg::new(reg),
                            val: s.len() as i64,
                        });
                        return Ok(TypedReg {
                            reg: AnyReg::Int(Reg::new(reg)),
                            ty: Ty::Int,
                        });
                    }
                    let TypedReg { reg: v, ty } = self.lower_expr(expr, None)?;
                    match ty {
                        Ty::Str => {
                            let s_reg = str_lane(v)?;
                            self.emit(Instruction::StrLen {
                                target: Reg::new(reg),
                                s: s_reg,
                            });
                            Ok(TypedReg {
                                reg: AnyReg::Int(Reg::new(reg)),
                                ty: Ty::Int,
                            })
                        }
                        Ty::Any => {
                            let a_reg = any_of(v)?;
                            self.emit(Instruction::AnyLen {
                                target: Reg::new(reg),
                                source: a_reg,
                            });
                            Ok(TypedReg {
                                reg: AnyReg::Int(Reg::new(reg)),
                                ty: Ty::Int,
                            })
                        }
                        Ty::Tbl(_) | Ty::Unknown(_) => {
                            let t_reg = ptr_of(v)?;
                            self.emit(Instruction::TableLen {
                                target: Reg::new(reg),
                                table: t_reg,
                            });
                            Ok(TypedReg {
                                reg: AnyReg::Int(Reg::new(reg)),
                                ty: Ty::Int,
                            })
                        }
                        _ => panic!(
                            "'#' requires a String, table, or Any operand — \
                             the checker rejects every other operand face \
                             before the lowerer runs"
                        ),
                    }
                }
            },
        }
    }
}
