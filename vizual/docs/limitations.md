# Current limitations

- Vizual exposes a breaking `0.1.0` API and several public underscore-style
  names.
- Winit is the only window/input backend, Vello the only renderer, and Parley
  the only text engine. An unsupported GPU produces an initialization error;
  there is no renderer fallback.
- The runner owns one resizable window with platform-default initial
  dimensions and requires an active Tokio runtime.
- Geometry is expressed in floating-point logical pixels. There are no public
  window, font, or display-scale configuration surfaces.
- Components are not automatically clipped. Only `Paragraph` and `Screen`
  select visible text ranges and expose scrolling.
- Layout has no overflow recovery. An undersized window reports the existing
  overconstrained-layout diagnostic instead of compressing or scrolling the
  component tree.
- `Text_input` intentionally omits clipboard and pointer selection.
- `Form` submission and exit behavior remains experimental.
- `Button` remains pointer-oriented rather than keyboard-focusable.
- `Screen` command execution uses `/bin/bash -c` on Unix and is unsupported
  on non-Unix systems.
- Public errors use `color_eyre::eyre::Result` rather than a structured crate
  error enum.
