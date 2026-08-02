# Repository guidelines

- Do not store widget instances when their state already lives elsewhere or stable slot identity is sufficient. Compose those widgets during `layout` instead.
- Do not introduce pointer-based or shared-ownership designs when ordinary ownership is sufficient; most solutions do not require pointers or `*` dereferences. Do not hide a necessary dereference behind `as_ref()`—when a pointer is genuinely required, an explicit dereference is clearer.
