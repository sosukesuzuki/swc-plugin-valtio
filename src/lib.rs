use swc_plugin::{ast::*, plugin_transform};

#[derive(Default)]
pub struct TransformVisitor {
    in_function: u32,
}

impl VisitMut for TransformVisitor {
    noop_visit_mut_type!();

    fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
        self.in_function = self.in_function + 1;
        fn_expr.body.visit_mut_children_with(self);
        self.in_function = self.in_function - 1;
    }

    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        self.in_function = self.in_function + 1;
        arrow_expr.body.visit_mut_children_with(self);
        self.in_function = self.in_function - 1;
    }

    fn visit_mut_call_expr(&mut self, call_expr: &mut CallExpr) {
        // in render
        if self.in_function == 1 {
            if let Callee::Expr(callee) = &call_expr.callee {
                if let Expr::Ident(callee_ident) = &**callee {
                    if &*callee_ident.sym == "useProxy" {
                        panic!("hohohoho");
                    }
                }
            }
        }
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
        "const Component = () => {};"
    );
}
