//! LLVM IR emission with aliasing and alignment metadata.

#[cfg(test)]
mod tests {
    use inkwell::context::Context;

    #[test]
    fn llvm_links() {
        let ctx = Context::create();
        let module = ctx.create_module("smoke");
        assert_eq!(module.get_name().to_str().unwrap(), "smoke");
    }
}
