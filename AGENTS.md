# Repository guidelines

- follow instructions precisely
- Do not store widget instances when their state already lives elsewhere or stable slot identity is sufficient. Compose those widgets during `layout` instead.
- Do not introduce pointer-based or shared-ownership designs when ordinary ownership is sufficient; most solutions do not require pointers or `*` dereferences. Do not hide a necessary dereference behind `as_ref()`—when a pointer is genuinely required, an explicit dereference is clearer.
- Do not add suffixed compatibility methods such as `solve_with_*` merely to preserve an old signature when the original method can accept the new context. Prefer updating crate-internal call sites and keeping the direct, unsuffixed name.
- Model new behavior as a local component before extending shared infrastructure. Avoid cross-cutting hooks that merely compensate for a missing abstraction.
- Do not create trivial wrapper macros or helper functions that merely forward to one constructor, such as `left_aligned(child)` around an anchor constructor. Put the expressive convenience constructor on the owning type instead, such as `Anchor::left(child)`.
- Do not avoid making methods async when needed.
- Calling layout of a widget manually is prohibited inside another widget as that will fail to create a child with its own events.
- put tests in a seperate tests.rs (where they must be in a directory named by the former `<file>.rs` ie. `<file>/tests.rs` where `<file>.rs` gets put in `<file>/mod.rs`)