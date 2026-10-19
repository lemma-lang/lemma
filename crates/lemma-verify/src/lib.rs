//! Type checking and proofs (bounds, spec/impl equivalence) via Z3.

#![deny(missing_docs)]

#[cfg(test)]
mod tests {
    use z3::ast::Int;
    use z3::{SatResult, Solver};

    #[test]
    fn z3_links() {
        let solver = Solver::new();
        let x = Int::new_const("x");
        solver.assert(x.gt(2));
        assert_eq!(solver.check(), SatResult::Sat);
    }
}
