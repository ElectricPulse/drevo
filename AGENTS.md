# Repository guidelines

- Do not store widget instances when their state already lives elsewhere or stable slot identity is sufficient. Compose those widgets during `layout` instead.
- Do not introduce pointer-based or shared-ownership designs when ordinary ownership is sufficient; most solutions do not require pointers or `*` dereferences. Do not hide a necessary dereference behind `as_ref()`—when a pointer is genuinely required, an explicit dereference is clearer.
- Do not add suffixed compatibility methods such as `solve_with_*` merely to preserve an old signature when the original method can accept the new context. Prefer updating crate-internal call sites and keeping the direct, unsuffixed name.
- Model new behavior as a local component before extending shared infrastructure. Avoid cross-cutting hooks that merely compensate for a missing abstraction.
- Do not create trivial wrapper macros or helper functions that merely forward to one constructor, such as `left_aligned(child)` around an anchor constructor. Put the expressive convenience constructor on the owning type instead, such as `Anchor::left(child)`.
- Do not avoid making methods async when needed.
- TODO: Fix the redundant component level that every `display!` adds to diagnostic component trees. Repeated same-source pairs such as `c28/c27` are logging noise from the extra wrapper, not meaningful hierarchy, and should not be accepted as the intended tree representation.
