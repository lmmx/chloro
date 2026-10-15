//! Scratch storage for the formatter's short-lived intermediates (EXP-025): a
//! [`bumpalo::Bump`] per thread, lent to a rewrite by [`with_scratch`] and reset at every
//! unit boundary of `visit_items_with_reordering` by [`reset_scratch`].
//!
//! Not a rustfmt module: rustfmt's modules allocate every intermediate with the global
//! allocator. Only values made and dropped inside one module are allocated here (a call's
//! list items, a chain's entries, a path's segments); a value that leaves its module stays a
//! `String`, so the `Option<String>` protocol between modules is unchanged.
//!
//! Every scratch value borrows the `&Bump` that `with_scratch` lends to its closure, so the
//! borrow checker proves that none outlives the closure; `reset` takes the bump mutably and
//! so can only run when no closure holds it (a nested unit boundary inside a rewrite, a
//! `mod` in a block in a closure in a chain, skips its reset).

use std::cell::RefCell;

pub(crate) use bumpalo::Bump;
pub(crate) use bumpalo::collections::{String as ScratchString, Vec as ScratchVec};

thread_local! {
    static SCRATCH: RefCell<Bump> = RefCell::new(Bump::new());
}

/// Lends the thread's bump to `f`; nothing allocated in it can be returned by `f`. When `f`
/// was the outermost borrower, nothing allocated in the bump is alive any more, and it is
/// reset (keeping its largest chunk).
#[inline]
pub(crate) fn with_scratch<R>(f: impl FnOnce(&Bump) -> R) -> R {
    SCRATCH.with(|bump| {
        let result = f(&bump.borrow());
        if let Ok(mut bump) = bump.try_borrow_mut() {
            bump.reset();
        }
        result
    })
}

/// Frees everything allocated in the thread's bump, keeping its largest chunk; skipped when
/// a rewrite up the stack holds the bump.
pub(crate) fn reset_scratch() {
    SCRATCH.with(|bump| {
        if let Ok(mut bump) = bump.try_borrow_mut() {
            bump.reset();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three users of the arena (chains, overflow's list items, types' path segments) on
    /// one input, formatted as rustfmt formats it.
    #[test]
    fn formatting_through_the_arena_users() {
        let src = "fn f() {\n    let result = some_receiver.method_one::<Vec<u8>>(first_argument, \
                   second_argument).method_two(std::collections::HashMap::<String, usize>::new(), \
                   InFile::new(file_id, decl))?.method_three();\n    acc.push(Foo { a: 1 }.into());\n}\n";
        let expected = "fn f() {
    let result = some_receiver
        .method_one::<Vec<u8>>(first_argument, second_argument)
        .method_two(
            std::collections::HashMap::<String, usize>::new(),
            InFile::new(file_id, decl),
        )?
        .method_three();
    acc.push(Foo { a: 1 }.into());
}
";
        assert_eq!(crate::format_source(src), expected);
        let used = SCRATCH.with(|bump| bump.borrow().allocated_bytes());
        assert!(used > 0, "the arena was used");
    }
}
