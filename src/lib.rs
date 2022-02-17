use swc_common::BytePos;
use swc_plugin::{ast::*, plugin_transform, syntax_pos::DUMMY_SP};

pub struct TransformVisitor {
    in_function: u32,
    proxy_name_and_span: Option<(JsWord, (BytePos, BytePos))>,
    snap_name: Option<JsWord>,
}

impl TransformVisitor {
    fn new() -> Self {
        Self {
            in_function: 0,
            proxy_name_and_span: None,
            snap_name: None,
        }
    }

    fn visit_mut_fn_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        // in render
        if self.in_function == 1 {
            // find `useProxy` call
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
                let first_arg = &args[0];
                let maybe_proxy_name_and_span = if let None = first_arg.spread {
                    if let Expr::Ident(ident) = &*first_arg.expr {
                        Some((ident.sym.clone(), (ident.span.lo, ident.span.hi)))
                    } else {
                        None
                    }
                } else {
                    None
                };
                // replace `useProxy(state)` with `const snap = useSnap(state);`.
                if let Some((proxy_name, proxy_span)) = maybe_proxy_name_and_span {
                    self.proxy_name_and_span = Some((proxy_name.clone(), proxy_span));
                    let snap_name: JsWord = format!("valtio_macro_snap_{}", proxy_name).into();
                    self.snap_name = Some(snap_name.clone());
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
                                    sym: snap_name,
                                },
                            }),
                            definite: false,
                            init: Some(Box::new(Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: Callee::Expr(Box::new(Expr::Ident(Ident {
                                    span: DUMMY_SP,
                                    optional: false,
                                    sym: "useSnapshot".into(),
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

    fn visit_mut_module_items_to_transform_import(&mut self, module_body: &mut Vec<ModuleItem>) {
        // find index of `import { useProxy } from "valtio/macro"`
        let maybe_use_proxy_import_idx =
            module_body
                .iter_mut()
                .enumerate()
                .find_map(|(idx, module_item)| match module_item {
                    ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                        if &*import_decl.src.value == "valtio/macro"
                            && import_decl
                                .specifiers
                                .iter()
                                .any(|specifier| match specifier {
                                    ImportSpecifier::Named(named_specifier) => {
                                        &*named_specifier.local.sym == "useProxy"
                                    }
                                    _ => false,
                                })
                        {
                            Some(idx)
                        } else {
                            None
                        }
                    }
                    _ => None,
                });
        if let Some(use_proxy_import_idx) = maybe_use_proxy_import_idx {
            // remove `import { useProxy } from "valtio/macro"`
            module_body.remove(use_proxy_import_idx);
            // insert `import { useSnapshot } from "valtio"`
            module_body.insert(
                0,
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    type_only: false,
                    asserts: None,
                    src: Str {
                        span: DUMMY_SP,
                        has_escape: false,
                        kind: StrKind::Normal {
                            contains_quote: false,
                        },
                        value: "valtio".into(),
                    },
                    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
                        span: DUMMY_SP,
                        is_type_only: false,
                        local: Ident {
                            span: DUMMY_SP,
                            sym: "useSnapshot".into(),
                            optional: false,
                        },
                        imported: None,
                    })],
                })),
            );
        }
    }

    fn visit_mut_ident_to_rename(&mut self, ident: &mut Ident) {
        // in render
        if self.in_function == 1 {
            if let Some((proxy_name, (proxy_span_lo, proxy_span_hi))) = &self.proxy_name_and_span {
                if let Some(snap_name) = &self.snap_name {
                    if &*ident.sym == &*proxy_name
                        && ident.span.lo != proxy_span_lo.clone()
                        && ident.span.hi != proxy_span_hi.clone()
                    {
                        ident.sym = snap_name.clone();
                    }
                }
            }
        }
    }
}

impl VisitMut for TransformVisitor {
    noop_visit_mut_type!();

    fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
        self.in_function += 1;
        if let Some(block_stmt) = &mut fn_expr.function.body {
            self.visit_mut_fn_stmts(&mut block_stmt.stmts);
        }
        fn_expr.visit_mut_children_with(self);
        self.in_function -= 1;
    }

    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        self.in_function += 1;
        if let BlockStmtOrExpr::BlockStmt(block_stmt) = &mut arrow_expr.body {
            self.visit_mut_fn_stmts(&mut block_stmt.stmts);
        }
        arrow_expr.visit_mut_children_with(self);
        self.in_function -= 1;
    }

    fn visit_mut_ident(&mut self, ident: &mut Ident) {
        self.visit_mut_ident_to_rename(ident);
        ident.visit_mut_children_with(self);
    }

    fn visit_mut_module(&mut self, module: &mut Module) {
        self.visit_mut_module_items_to_transform_import(&mut module.body);
        module.visit_mut_children_with(self);
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
    program.fold_with(&mut as_folder(TransformVisitor::new()))
}

#[cfg(test)]
mod transform_visitor_tests {
    use swc_ecma_transforms_testing::test;

    use super::*;

    fn transform_visitor() -> impl 'static + Fold + VisitMut {
        as_folder(TransformVisitor::new())
    }

    test!(
        ::swc_ecma_parser::Syntax::Es(::swc_ecma_parser::EsConfig {
            jsx: true,
            ..Default::default()
        }),
        |_| transform_visitor(),
        use_proxy_macros,
        r#"
        import { useProxy } from 'valtio/macro'
        const Component = () => {
          useProxy(state)
          return <div>
            {state.count}
            <button onClick={() => ++state.count}>+1</button>
          </div>
        }
        "#,
        r#"
        import { useSnapshot } from 'valtio';
        const Component = () => {
          const valtio_macro_snap_state = useSnapshot(state);
          return <div>
            {valtio_macro_snap_state.count}
            <button onClick={() => ++state.count}>+1</button>
          </div>
        }
        "#
    );
}
