impl Checker {

    fn check_ffi_call(
        &mut self,
        lat: &mut UnknownStore,
        call: &Expr,
        obj: &Expr,
        name: &str,
        args: &[Expr],
    ) -> Result<Ty, Diagnostic> {
        let base = match obj {
            Expr::Identifier { name: n, .. } => n.clone(),
            _ => {
                return Err(Diagnostic::ty(
                    "the ffi surface's '.' base must be a name — a bound \
                     module or library handle",
                ));
            }
        };
        let raw = self.check_expr(lat, obj)?;
        let bt = lat.resolve(&raw);
        match bt {
            Ty::Ffi => match name {
                "load" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty("ffi.load takes exactly one path"));
                    }
                    let t = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&t);
                    let t = if let Ty::Unknown(id) = t {
                        lat.bind(id, Ty::Str)?;
                        Ty::Str
                    } else {
                        t
                    };
                    match t {
                        Ty::Str => {
                            self.record_verb(call, FfiVerb::Load);
                            Ok(Ty::FfiLib)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "ffi.load's path is a String, got {}",
                            type_name(&other)
                        ))),
                    }
                }
                "cdef" => Err(Diagnostic::ty(
                    "ffi.cdef is a statement verb — call it on its own line; its \
                     nothing cannot bind",
                )),
                "arg_kind" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty("ffi.arg_kind takes exactly one library"));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    match t {
                        Ty::FfiLib => {
                            self.record_verb(call, FfiVerb::ArgKind);
                            Ok(Ty::Int)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "ffi.arg_kind takes a library handle, got {}",
                            type_name(&other)
                        ))),
                    }
                }
                _ => Err(Diagnostic::ty(format!(
                    "unknown ffi member '{name}' — load, cdef, arg_kind"
                ))),
            },
            Ty::Coro => match name {
                "create" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty("coroutine.create takes exactly one library"));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    match t {
                        Ty::FfiLib => {
                            self.record_verb(call, FfiVerb::CoroCreate);
                            Ok(Ty::Int)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "coroutine.create takes a library handle, got {}",
                            type_name(&other),
                        ))),
                    }
                }
                "resume" => {
                    if args.len() != 2 {
                        return Err(Diagnostic::ty(
                            "coroutine.resume takes a coroutine handle and one \
                             table — the spawn inputs (its first three cells ride the \
                             worker's boundary)",
                        ));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    let t = if let Ty::Unknown(id) = t {
                        lat.bind(id, Ty::Int)?;
                        Ty::Int
                    } else {
                        t
                    };
                    match t {
                        Ty::Int => {}
                        other => {
                            return Err(Diagnostic::ty(format!(
                                "coroutine.resume's first argument is a \
                                 coroutine handle, got {}",
                                type_name(&other)
                            )));
                        }
                    }
                    let raw = self.check_expr(lat, &args[1])?;
                    let t = lat.resolve(&raw);
                    match t {
                        Ty::Tbl(_) => {
                            if self.carries_closure(&args[1]) {
                                return Err(Diagnostic::ty(
                                    "a closure cannot cross a module \
                                         boundary — the spawn table marshals flat cells only",
                                ));
                            }
                            self.record_verb(call, FfiVerb::CoroResume);
                            Ok(Ty::Int)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "coroutine.resume's second argument is the input \
                                 table, got {}",
                            type_name(&other),
                        ))),
                    }
                }
                "yield" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty("coroutine.yield takes exactly one integer"));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    let t = if let Ty::Unknown(id) = t {
                        lat.bind(id, Ty::Int)?;
                        Ty::Int
                    } else {
                        t
                    };
                    match t {
                        Ty::Int => {
                            self.record_verb(call, FfiVerb::CoroYield);
                            Ok(Ty::Int)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "coroutine.yield streams one integer, got {}",
                            type_name(&other),
                        ))),
                    }
                }
                "status" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty(
                            "coroutine.status takes exactly one coroutine",
                        ));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    let t = if let Ty::Unknown(id) = t {
                        lat.bind(id, Ty::Int)?;
                        Ty::Int
                    } else {
                        t
                    };
                    match t {
                        Ty::Int => {
                            self.record_verb(call, FfiVerb::CoroStatus);
                            Ok(Ty::Str)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "coroutine.status takes a coroutine handle, got {}",
                            type_name(&other),
                        ))),
                    }
                }
                "close" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::ty(
                            "coroutine.close takes exactly one coroutine",
                        ));
                    }
                    let raw = self.check_expr(lat, &args[0])?;
                    let t = lat.resolve(&raw);
                    let t = if let Ty::Unknown(id) = t {
                        lat.bind(id, Ty::Int)?;
                        Ty::Int
                    } else {
                        t
                    };
                    match t {
                        Ty::Int => {
                            self.record_verb(call, FfiVerb::CoroClose);
                            Ok(Ty::Int)
                        }
                        other => Err(Diagnostic::ty(format!(
                            "coroutine.close takes a coroutine handle, got {}",
                            type_name(&other),
                        ))),
                    }
                }
                _ => Err(Diagnostic::ty(format!(
                    "unknown coroutine member '{name}' — create, resume, yield, \
                     status, close"
                ))),
            },
            Ty::FfiLib => Err(Diagnostic::ty(format!(
                "{base}.{name} is the drive verb — a statement on its own line; \
                 the module's result stays in the module's world, nothing comes back to bind"
            ))),
            _ => Err(Diagnostic::ty(format!(
                "'{base}' is not a module surface — '.' is the ffi/coroutine \
                 surface's, tables use '[]'"
            ))),
        }
    }

    fn check_ffi_verb(&mut self, lat: &mut UnknownStore, expr: &Expr) -> Result<bool, Diagnostic> {
        let Expr::Call { callee, args, .. } = expr else {
            return Ok(false);
        };
        let Expr::Field { obj, name, .. } = callee.as_ref() else {
            return Ok(false);
        };
        let Expr::Identifier { name: base, .. } = obj.as_ref() else {
            return Ok(false);
        };
        let raw = self.check_expr(lat, obj)?;
        let bt = lat.resolve(&raw);
        match (&bt, name.as_str()) {
            (Ty::Ffi, "cdef") => {
                if args.len() != 1 {
                    return Err(Diagnostic::ty(
                        "ffi.cdef takes exactly one declaration string",
                    ));
                }
                let raw = self.check_expr(lat, &args[0])?;
                let t = lat.resolve(&raw);
                match t {
                    Ty::Str => {
                        self.record_verb(expr, FfiVerb::Cdef);
                        Ok(true)
                    }
                    other => Err(Diagnostic::ty(format!(
                        "ffi.cdef's declaration is a String, got {}",
                        type_name(&other)
                    ))),
                }
            }
            (Ty::FfiLib, _) => {
                if args.len() != 1 {
                    return Err(Diagnostic::ty(format!(
                        "the drive call .{name} takes exactly one table — the \
                         boundary the module reads"
                    )));
                }
                let raw = self.check_expr(lat, &args[0])?;
                let t = lat.resolve(&raw);
                match t {
                    Ty::Tbl(_) => {
                        if self.carries_closure(&args[0]) {
                            return Err(Diagnostic::ty(
                                "a closure cannot cross a module boundary — \
                                 the drive table marshals flat cells only",
                            ));
                        }
                        self.record_verb(
                            expr,
                            FfiVerb::Drive {
                                base: base.clone(),
                                sym: name.clone(),
                            },
                        );
                        Ok(true)
                    }
                    other => Err(Diagnostic::ty(format!(
                        "the drive call .{name} takes one table, got {}",
                        type_name(&other)
                    ))),
                }
            }
            _ => Ok(false),
        }
    }
}
