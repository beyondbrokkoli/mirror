impl<'a> IrLowerer<'a> {

    fn lookup_layout_for(&self, expr: &Expr) -> Result<LayoutVerdict, LowerError> {
        match expr {
            Expr::TableCtor { .. } => {
                if let Some(&site_id) = self.shape.sites.get(&expr.id()) {
                    Ok(self.shape.layouts[site_id])
                } else {
                    Ok(LayoutVerdict::default())
                }
            }
            Expr::Identifier { name, .. } => Ok(self.read_var(name)?.layout),
            Expr::Index { obj, .. } => self.lookup_layout_for(obj),
            _ => Ok(LayoutVerdict::default()),
        }
    }

    /// The M2 builtins, lowered straight from the checker's recorded
    /// verb — assert guards its condition, error raises through the
    /// trap, tonumber parses the pooled digits (a literal folds here,
    /// no runtime call).
    fn lower_builtin_call(
        &mut self,
        verb: BuiltinVerb,
        args: &[Expr],
        reg: RegId,
    ) -> Result<TypedReg, LowerError> {
        match verb {
            BuiltinVerb::Assert => {
                let TypedReg { reg: c, .. } = self.lower_expr(&args[0], None)?;
                let cond = bool_of(c)?;
                let msg = if let Some(msg_expr) = args.get(1) {
                    let TypedReg { reg: m, .. } = self.lower_expr(msg_expr, None)?;
                    str_lane(m)?
                } else {
                    // No message given — the default rides the strtab
                    // like any literal, so the trap reads a pooled
                    // handle either way.
                    let m = self.next_reg();
                    self.emit(Instruction::LoadString {
                        target: Reg::new(m),
                        val: "assertion failed!".to_string(),
                    });
                    Reg::new(m)
                };
                self.emit(Instruction::AssertGuard { cond, msg });
                Ok(TypedReg {
                    reg: AnyReg::Bool(cond),
                    ty: Ty::Bool,
                })
            }
            BuiltinVerb::Error => {
                let TypedReg { reg: m, .. } = self.lower_expr(&args[0], None)?;
                let msg = str_lane(m)?;
                self.emit(Instruction::ErrorRaise { msg });
                Ok(TypedReg {
                    reg: AnyReg::Str(msg),
                    ty: Ty::Str,
                })
            }
            BuiltinVerb::ToNumber => {
                if let Expr::String { value, .. } = &args[0] {
                    let n: i64 = value
                        .parse()
                        .expect("the checker rejected non-digit literals");
                    self.emit(Instruction::LoadInt {
                        target: Reg::new(reg),
                        val: n,
                    });
                    return Ok(TypedReg {
                        reg: AnyReg::Int(Reg::new(reg)),
                        ty: Ty::Int,
                    });
                }
                let TypedReg { reg: s, .. } = self.lower_expr(&args[0], None)?;
                let s = str_lane(s)?;
                self.emit(Instruction::StrToNumber {
                    target: Reg::new(reg),
                    s,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
        }
    }

                                fn lower_ffi_call(
        &mut self,
        call: &Expr,
        args: &[Expr],
        reg: RegId,
    ) -> Result<TypedReg, LowerError> {
        let verb = self
            .shape
            .ffi_verbs
            .get(&call.id())
            .cloned()
            .expect(
                "a dotted call site carries no resolved verb — the \
                 checker records a verb for every checker-accepted \
                 dotted call",
            );
        signal!(trace::TRACE_LOWER_FFI_VERB);
        match verb {
            FfiVerb::Load => {
                let path = self.lower_expr(&args[0], None)?;
                let path_reg = pooled_ptr_of(path.reg)?;
                self.emit(Instruction::FfiLoad {
                    target: Reg::new(reg),
                    path: path_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::FfiLib,
                })
            }
            FfiVerb::ArgKind => {
                let lib = self.lower_expr(&args[0], None)?;
                let lib_reg = pooled_ptr_of(lib.reg)?;
                self.emit(Instruction::FfiArgKind {
                    target: Reg::new(reg),
                    lib: lib_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::Cdef => {
                // W4: the declaration's effect is entirely
                // compile-time — registry rows were consumed before
                // emission. The LoadNull placeholder is gone; the
                // statement lowers to nothing.
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Ffi,
                })
            }
            FfiVerb::New { .. } => {
                signal!(trace::TRACE_LOWER_FFI_NEW);
                let FfiMint { elem, capacity, .. } = self
                    .shape
                    .ffi_mints
                    .get(&call.id())
                    .cloned()
                    .expect(
                        "a checker-accepted ffi.new carries its mint fact — \
                         elem and capacity recorded at the choke point",
                    );
                self.emit(Instruction::TableNew {
                    target: Reg::new(reg),
                    elem: elem.clone(),
                    flags: 0,
                });
                let cap_reg = self.next_reg();
                self.emit(Instruction::LoadInt {
                    target: Reg::new(cap_reg),
                    val: capacity,
                });
                self.emit(Instruction::TableReserve {
                    table: Reg::new(reg),
                    bound: Reg::new(cap_reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::Tbl(Box::new(elem)),
                })
            }
            FfiVerb::Sizeof { bytes } => {
                self.emit(Instruction::LoadInt {
                    target: Reg::new(reg),
                    val: bytes,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::CoroCreate => {
                let lib = self.lower_expr(&args[0], None)?;
                let lib_reg = pooled_ptr_of(lib.reg)?;
                self.emit(Instruction::CoroCreate {
                    target: Reg::new(reg),
                    lib: lib_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::CoroResume => {
                let handle = self.lower_expr(&args[0], None)?;
                let handle_reg = int_of(handle.reg)?;
                let tbl = self.lower_expr(&args[1], None)?;
                let tbl_reg = ptr_of(tbl.reg)?;
                self.emit(Instruction::CoroResume {
                    target: Reg::new(reg),
                    handle: handle_reg,
                    args: tbl_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::CoroYield => {
                let value = self.lower_expr(&args[0], None)?;
                let value_reg = int_of(value.reg)?;
                self.emit(Instruction::CoroYield {
                    target: Reg::new(reg),
                    value: value_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::CoroStatus => {
                let handle = self.lower_expr(&args[0], None)?;
                let handle_reg = int_of(handle.reg)?;
                self.emit(Instruction::CoroStatus {
                    target: Reg::new(reg),
                    handle: handle_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Str(Reg::new(reg)),
                    ty: Ty::Str,
                })
            }
            FfiVerb::CoroClose => {
                let handle = self.lower_expr(&args[0], None)?;
                let handle_reg = int_of(handle.reg)?;
                self.emit(Instruction::CoroClose {
                    target: Reg::new(reg),
                    handle: handle_reg,
                });
                Ok(TypedReg {
                    reg: AnyReg::Int(Reg::new(reg)),
                    ty: Ty::Int,
                })
            }
            FfiVerb::Drive { base, sym } => {
                let local = self.read_var(&base)?;
                let lib_reg = pooled_ptr_of(local.reg)?;
                let arg = self.lower_expr(&args[0], None)?;
                let arg_reg = ptr_of(arg.reg)?;
                self.emit(Instruction::FfiCall {
                    lib: lib_reg,
                    sym,
                    arg: arg_reg,
                });
                self.emit(Instruction::LoadNull {
                    target: Reg::new(reg),
                });
                Ok(TypedReg {
                    reg: AnyReg::Ptr(Reg::new(reg)),
                    ty: Ty::Ffi,
                })
            }
        }
    }
}
