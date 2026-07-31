# Parameter Workspace Redesign

Date: 2026-07-31

## Goal

Redesign the parameter workspace of `local-ai-launcher-rs` so that current
`llama-server` parameters are accurate, common settings are easy to scan, and
advanced users can reorganize parameters without losing typed controls or
per-preset values.

The application remains Rust 2021 with `eframe/egui`. WinUI 3 and WinUI Gallery
are visual and interaction references, not a framework migration.

## Scope

- Replace deprecated mmap/mlock command generation with `--load-mode`.
- Add typed controls for Flash Attention and reasoning preservation.
- Fix speculative decoding choices to the values supported by the verified
  local `llama-server` build.
- Replace the current parameter subsections with Common, Other Parameters, and
  Extra Parameters.
- Add global drag-and-drop layout customization for built-in parameters.
- Replace the multiline extra-arguments editor with individually managed raw
  command-line rows.
- Preserve existing preset behavior through explicit configuration migration.
- Move the presentation toward a restrained Windows/WinUI settings style.

This change does not attempt to expose every `llama-server` argument, migrate
the application to WinUI, or dynamically scrape parameter definitions at
runtime. Unsupported or future arguments remain available through raw extra
parameter rows.

## Verified llama.cpp Parameters

The local verification target is:

- Executable: `llama-server.exe`
- Build: `10199` (`b4ca032ae`)
- Platform: Windows x86_64, Clang 20.1.8

The local `--help`, the official `common/arg.cpp`, and the official server README
agree on the following definitions:

- `-lm, --load-mode MODE`: `none`, `mmap`, `mlock`, `mmap+mlock`, or `dio`.
  llama.cpp defaults to `mmap` when the argument is omitted.
- `--mmap`, `--no-mmap`, and `--mlock` are deprecated in favor of load mode.
- `-fa, --flash-attn [on|off|auto]` is the Flash Attention option. There is no
  `-fan` option.
- `--reasoning-preserve` and `--no-reasoning-preserve` preserve reasoning in
  full chat history for templates that advertise
  `supports_preserve_reasoning`. The template default applies when omitted.
- `--cpu-moe` places all MoE expert weights on CPU. `--n-cpu-moe N` places the
  first N MoE layers on CPU. Their meanings overlap.
- `--image-min-tokens` and `--image-max-tokens` apply to vision models with
  dynamic image resolution.

The fixed speculative type choices are:

1. `none`
2. `draft-simple`
3. `draft-eagle3`
4. `draft-mtp`
5. `draft-dflash`
6. `draft-dspark`
7. `ngram-simple`
8. `ngram-map-k`
9. `ngram-map-k4v`
10. `ngram-mod`
11. `ngram-cache`

Every choice control also contains a launcher-only `Default` choice. `Default`
means that the launcher emits no argument; it is distinct from explicitly
selecting `none` or `auto`.

## Parameter Registry

Introduce one built-in parameter registry. Each definition contains:

- A stable parameter ID.
- Chinese display name and help text.
- Control kind: text, fixed choice, positive toggle, or two-sided toggle.
- Canonical long command-line name and optional negative name.
- Fixed choices where applicable.
- Default section and default order.

The registry drives rendering and command generation so the UI cannot silently
drift away from the actual argument spelling. `Preset` remains the strong-typed
value store for this change. A built-in parameter ID maps to the appropriate
field through explicit Rust matches; values are not converted into arbitrary
JSON.

Toggle controls have these states:

- Positive-only flags: `Default`, `Enabled`.
- Flags with an official negative form: `Default`, `Enabled`, `Disabled`.

`Default` emits nothing. `Enabled` emits the positive long name. `Disabled`
emits the official negative long name.

## Default Sections

### Common

- Listen address
- Port
- Model alias
- GPU layers
- CPU MoE layers
- Threads
- Batch size
- Micro-batch size
- Parallel slots
- Context size
- K cache type
- V cache type
- Load mode
- Flash Attention
- Speculative type
- Draft maximum N
- Draft minimum N
- Image minimum tokens
- Image maximum tokens
- Enable Jinja template

### Other Parameters

- Timeout
- Multi-GPU split mode
- Tensor split
- Main GPU
- Device
- Draft minimum probability
- Draft split probability
- llama.cpp Web UI
- Log timestamps
- Offline mode
- Verbose logging
- KV cache offload
- Unified KV buffer
- Full-size SWA cache
- All MoE on CPU
- Preserve reasoning

### Extra Parameters

This section contains raw user-defined command-line rows and the Add Parameter
command. A raw row may later be moved to Common or Other Parameters without
changing its behavior.

## New Preset Defaults

- Jinja starts at `Enabled` and emits `--jinja`.
- llama.cpp Web UI starts at `Default`.
- Log timestamps starts at `Default`.
- All other toggle and choice controls introduced by this redesign start at
  `Default`.
- Existing text and numeric defaults remain unchanged.

## Specific Parameter Behavior

### Load Mode

The choice list is `Default`, `none`, `mmap`, `mlock`, `mmap+mlock`, and `dio`.
The launcher emits `--load-mode VALUE` only for a non-default selection.

### Flash Attention

The choice list is `Default`, `on`, `off`, and `auto`. The launcher emits the
canonical form `--flash-attn VALUE`.

### Reasoning Preservation

This is a two-sided toggle in Other Parameters:

- `Default`: emit nothing.
- `Enabled`: emit `--reasoning-preserve`.
- `Disabled`: emit `--no-reasoning-preserve`.

### Speculative Decoding

The speculative type control contains `Default` plus the eleven verified
values. If the launcher detects an MTP draft model while the control remains at
`Default`, it emits only `--spec-type draft-mtp`. An explicit user selection,
including `none`, takes priority.

MTP detection no longer injects draft maximum N, draft minimum N, or draft
probability arguments. Those remain omitted at `Default`, allowing llama.cpp to
apply its own defaults.

### CPU MoE

When All MoE on CPU is `Enabled`, the CPU MoE layers input is disabled but its
stored value is retained. Command generation emits only `--cpu-moe` and
suppresses `--n-cpu-moe`. Returning All MoE on CPU to `Default` restores the
layer input and its prior value.

## Layout Customization

Normal mode presents a compact WinUI-inspired settings surface without drag
handles. The parameter panel has three top-level sections: Common is expanded,
Other Parameters is collapsible, and Extra Parameters contains the add command
and raw rows.

A `Customize layout` command enters a dedicated edit state:

- Built-in rows show drag handles.
- Common and Other Parameters become visible drop targets.
- Raw rows can move among all three sections.
- `Restore default` restores the registry's built-in section and order.
- `Done` leaves customization mode.

Built-in section and order are stored globally and apply to every preset. Raw
row text, enabled state, section, and order are stored per preset. Moving a raw
row in one preset does not introduce it into another preset.

Dragging affects presentation only. Built-in arguments retain a canonical
generation order. Enabled raw rows are appended after built-in arguments in
their stored relative order so advanced users can intentionally override an
earlier built-in argument.

## Raw Extra Parameters

Add Parameter creates one editable, single-line raw argument item. Each item
has an enabled state, text editor, delete command, and drag affordance in layout
customization mode. Examples include:

```text
--flash-attn on
--tensor-split 3,1
--alias "Local Model"
```

Empty rows are ignored during command generation. The existing quote-aware
argument splitter remains the parsing basis. Unclosed quotes or an unsplittable
row produce an inline warning for that row; they do not prevent other rows from
being saved or previewed.

## WinUI-Inspired Presentation

The redesign follows the interaction language demonstrated by WinUI 3 and
WinUI Gallery while staying within egui:

- Compact standard-height fields and buttons.
- Clear section hierarchy with restrained borders and neutral surfaces.
- Conventional combo boxes for choices and icon commands for row actions.
- A visible edit state instead of permanent drag decoration.
- Consistent spacing, focus treatment, disabled states, and keyboard access.
- No nested decorative cards or oversized rounded controls.

The existing three-column application workspace remains. This change is scoped
to the parameter panel and shared control styling needed by that panel.

## Configuration Migration

Configuration loading performs a versioned migration before normal
deserialization. Existing preset values and effective behavior are preserved.

Legacy mmap/mlock pairs map by intended combined behavior:

| Legacy mmap | Legacy mlock | New load mode |
| --- | --- | --- |
| false | false | `none` |
| true | false | `mmap` |
| false | true | `mlock` |
| true | true | `mmap+mlock` |

Legacy booleans map as follows:

- A prior `true` maps to `Enabled`.
- A prior `false` for a positive-only flag maps to `Default`.
- A prior `false` for Web UI, log timestamps, or KV offload maps to `Disabled`
  because the old command builder explicitly emitted the negative argument.
- Legacy Jinja `false` maps to `Default`.

Each non-empty line in legacy `extra_args` becomes one enabled raw row in Extra
Parameters, preserving line order. Missing layout data uses the new registry
defaults. Migration writes no model paths or machine-specific information to
new locations.

## Command and Export Consistency

One final argument vector is used by server launch, command preview, and bat
export. Built-in arguments use canonical long names. Existing quoting rules for
model, draft model, and mmproj paths remain. Raw rows are parsed and appended
after built-in arguments.

The command preview is the authoritative visible representation of what the
launcher will execute.

## Error Handling

- Unknown built-in layout IDs are ignored while known IDs are retained.
- Missing built-in IDs are appended using registry default order so an upgrade
  can add parameters without resetting user layout.
- Duplicate built-in layout IDs are de-duplicated during normalization.
- Invalid fixed-choice values loaded from configuration fall back to `Default`.
- Malformed raw rows show a local warning and are excluded from the final
  command until corrected.
- Failed configuration migration returns the existing load error path rather
  than silently discarding the entire configuration.

## Verification

Automated tests cover:

- New preset defaults.
- Positive-only and two-sided toggle emission.
- Load mode and Flash Attention choices.
- Every fixed speculative type.
- MTP detection emitting only `--spec-type draft-mtp` at `Default`.
- Explicit speculative type overriding MTP detection.
- CPU MoE suppression of CPU MoE layers.
- Raw row parsing, enabled state, and relative order.
- Legacy boolean, mmap/mlock, extra argument, and layout migration.
- Command preview and bat export using the same final arguments.

Manual acceptance covers:

- Normal and Customize layout states.
- Reordering, cross-section dragging, restore-default, and persistence.
- Global built-in layout versus per-preset raw row placement.
- WinUI-inspired focus, disabled, hover, and long-text behavior.
- No overlap or clipping at the existing desktop window size.
- Help text and fixed choices matching `llama-server b10199 --help`.
- One real local model launch and stop using the current Windows llama.cpp
  distribution.

## Primary Files

- `src/config.rs`: toggle/value types, layout data, raw rows, and migration.
- `src/command.rs`: registry-driven argument emission and tests.
- `src/app.rs`: three-section panel, controls, layout customization, drag/drop,
  inline warnings, and WinUI-inspired styling.
- `README.md`: user-facing description of the new parameter workspace and
  current llama.cpp parameter names.

## References

- llama.cpp argument definitions:
  https://github.com/ggml-org/llama.cpp/blob/master/common/arg.cpp
- llama.cpp server parameter reference:
  https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
- WinUI 3 overview:
  https://learn.microsoft.com/zh-cn/windows/apps/winui/winui3/
- WinUI Gallery:
  https://github.com/microsoft/WinUI-Gallery
