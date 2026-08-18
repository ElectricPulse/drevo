# Vizual Architecture

Vizual is a component-based Rust UI framework built around fine-grained reactive state management, an asynchronous component lifecycle, and a constraint-based Mixed-Integer Linear Programming (MILP) layout solver.

---

## 1. Granular State Management

Rather than adopting a global Elm-style reducer loop or diffing a virtual DOM across the entire application tree, Vizual uses fine-grained, localized reactivity powered by `Store<T>` and `State<T>`.

- **Automatic Per-Component Subscriptions**: When a component reads state during layout or render passes via `store.affect(render).await`, the framework records a direct dependency between that specific component's slot and the store.
- **Targeted Re-renders**: When a store is mutated with `store.write().await?`, `Render_manager` schedules layout and render passes *only* for components that explicitly subscribed to that piece of state.
- **Deduplication**: Multiple state mutations within a frame are coalesced into a single, deduplicated pass.

---

## 2. MILP Constraint Layouter

Vizual formulates UI layout as a mathematical optimization problem solved using Mixed-Integer Linear Programming (MILP) with lexicographical multi-objective priorities, inspired by Cassowary and iOS Auto Layout.

- **Mathematical Geometric Constraints**: Widgets express relationships (hitbox bounds, axis flows, grid alignments, anchors, and padding) as linear equations and inequalities over edge variables (`start`, `end`, `origin`, `size`).
- **Lexicographical Priorities**: Constraints are solved hierarchically. Hard layout requirements (parent boundary containment, minimal element sizes) take highest priority, while soft objectives (dynamic spacing, stretch factors) resolve remaining degrees of freedom lexicographically without conflict.
- **Content-Driven Dynamic Resolution**: Solves complex layout feedback loops naturally—such as text paragraphs measuring and wrapping height based on dynamically solved available width constraints.

---

## 3. Component Model & Tuple Composition

- **Heterogeneous Tuple Layout**: Multi-child layout containers (`Axis`, `Grid`) accept tuples of arbitrary concrete widget types directly (e.g. `Axis::new(Direction::Vertical, (header, body, footer))`) via the `Into_widgets` trait, avoiding manual `Box::new` calls or homogeneous `Vec` boilerplate.
- **Dynamic Collections**: When child elements are dynamic at runtime (such as iterating over variable-length lists in menus or logs), containers also accept `Vec<Widget>`.
- **Async Component Lifecycle**: Components implement `Widget_trait` with asynchronous `layout` and `render` methods, allowing widgets to fetch or coordinate async resources during layout resolution.

---

## 4. Event Routing & Focus System

- **Focus Tree Hierarchy**: Keyboard navigation (`Tab`, `Shift + Tab`, arrow keys) traverses the active component hierarchy.
- **Click & Interaction Requirement**: To receive click and pointer events, a component must currently be able to participate in focus (`focus.set_interactive(true)` or focusable blocks). Pointer interactions route directly through the resolved focus and hitbox tree.
