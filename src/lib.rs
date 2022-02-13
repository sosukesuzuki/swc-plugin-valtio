use swc_plugin::{ast::*, plugin_transform, syntax_pos::DUMMY_SP};

#[derive(Default)]
pub struct TransformVisitor {
    in_function: u32,
}

impl TransformVisitor {
    fn visit_mut_fn_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        // in render
        if self.in_function == 1 {
            let maybe_use_proxy_info = stmts.iter_mut().enumerate().find_map(|(idx, stmt)| {
                if let Stmt::Expr(expr_stmt) = stmt {
                    if let Expr::Call(call_expr) = &*expr_stmt.expr {
                        if let Callee::Expr(callee) = &call_expr.callee {
                            if let Expr::Ident(callee_ident) = &**callee {
                                if &*callee_ident.sym == "useProxy" {
                                    Some((idx, &call_expr.args))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            if let Some((idx, args)) = maybe_use_proxy_info {
                stmts[idx] = Stmt::Decl(Decl::Var(VarDecl {
                    span: DUMMY_SP,
                    kind: VarDeclKind::Const,
                    declare: false,
                    decls: vec![VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(BindingIdent {
                            type_ann: None,
                            id: Ident {
                                span: DUMMY_SP,
                                optional: false,
                                sym: "snap".into(),
                            },
                        }),
                        definite: false,
                        init: Some(Box::new(Expr::Call(CallExpr {
                            span: DUMMY_SP,
                            callee: Callee::Expr(Box::new(Expr::Ident(Ident {
                                span: DUMMY_SP,
                                optional: false,
                                sym: "useSnap".into(),
                            }))),
                            type_args: None,
                            args: args.to_vec(),
                        }))),
                    }],
                }));
            }
        }
    }
}

impl VisitMut for TransformVisitor {
    noop_visit_mut_type!();

    fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
        self.in_function = self.in_function + 1;
        if let Some(block_stmt) = &mut fn_expr.function.body {
            self.visit_mut_fn_stmts(&mut block_stmt.stmts);
        }
        self.in_function = self.in_function - 1;
    }

    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        self.in_function = self.in_function + 1;
        if let BlockStmtOrExpr::BlockStmt(block_stmt) = &mut arrow_expr.body {
            self.visit_mut_fn_stmts(&mut block_stmt.stmts);
        }
        self.in_function = self.in_function - 1;
    }
}

/// An entrypoint to the SWC's transform plugin.
/// `plugin_transform` macro handles necessary interop to communicate with the host,
/// and entrypoint function name (`process_transform`) can be anything else.
///
/// If plugin need to handle low-level ptr directly,
/// it is possible to opt out from macro by writing transform fn manually via raw interface
///
/// `__plugin_process_impl(
///     ast_ptr: *const u8,
///     ast_ptr_len: i32,
///     config_str_ptr: *const u8,
///     config_str_ptr_len: i32) ->
///     i32 /*  0 for success, fail otherwise.
///             Note this is only for internal pointer interop result,
///             not actual transform result */
///
/// However, this means plugin author need to handle all of serialization/deserialization
/// steps with communicating with host. Refer `swc_plugin_macro` for more details.
#[plugin_transform]
pub fn process_transform(program: Program, _plugin_config: String) -> Program {
    program.fold_with(&mut as_folder(TransformVisitor::default()))
}

#[cfg(test)]
mod transform_visitor_tests {
    use swc_ecma_transforms_testing::test;

    use super::*;

    fn transform_visitor() -> impl 'static + Fold + VisitMut {
        as_folder(TransformVisitor::default())
    }

    test!(
        ::swc_ecma_parser::Syntax::default(),
        |_| transform_visitor(),
        use_proxy_macros,
        "const Component = () => {useProxy(state)};",
        "const Component = () => {const snap = useSnap(state)};"
    );
}
