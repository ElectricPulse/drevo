# Repository guidelines

- follow instructions precisely
- Do not store widget instances when their state already lives elsewhere or stable slot identity is sufficient. Compose those widgets during `layout` instead.
- Do not introduce pointer-based or shared-ownership designs when ordinary ownership is sufficient; most solutions do not require pointers or `*` dereferences. Do not hide a necessary dereference behind `as_ref()`—when a pointer is genuinely required, an explicit dereference is clearer.
- Do not add suffixed compatibility methods such as `solve_with_*` merely to preserve an old signature when the original method can accept the new context. Prefer updating crate-internal call sites and keeping the direct, unsuffixed name.
- Model new behavior as a local component before extending shared infrastructure. Avoid cross-cutting hooks that merely compensate for a missing abstraction.
- Do not create trivial wrapper macros or helper functions that merely forward to one constructor, such as `left_aligned(child)` around an anchor constructor. Put the expressive convenience constructor on the owning type instead, such as `Anchor::left(child)`.
- Do not avoid making methods async when needed.
- Calling layout of a widget manually is prohibited inside another widget as that will fail to create a child with its own events.
- put tests in a separate tests.rs (where they must be in a directory named by the former `<file>.rs` ie. `<file>/tests.rs` where `<file>.rs` gets put in `<file>/mod.rs`)
- proactively fix typos in code, comments, and documentation

# Documentation

Write documentation for developers who want to use or understand the code, not as marketing material.

Prefer plain, concrete language. Describe what something does before explaining why the architecture is interesting.

Keep paragraphs short. Remove sentences that do not add information.

Prefer:

Store<T> tracks which widgets read a value. When the value changes, those widgets are scheduled for another pass.

over:

Vizual provides fine-grained reactive state management through a sophisticated automatic dependency tracking system.

Do not add adjectives such as "powerful", "flexible", "robust", "elegant", "seamless", "advanced", or "sophisticated" unless they communicate a specific technical fact.

Do not invent names for concepts merely to make the documentation sound more formal. Use the terminology that exists in the code.

Avoid headings for every minor idea. A short paragraph is often better than a hierarchy of headings and bullet points.