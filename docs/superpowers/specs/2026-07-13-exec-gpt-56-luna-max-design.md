# `exec` Default Model and Reasoning Effort

## Problem

`codex-potter exec` does not currently provide an explicit default for the upstream model or
reasoning effort. As a result, an `exec` task cannot reliably select the `gpt-5.6-luna` model at
the `max` reasoning level. The local protocol and TUI configuration parser also do not recognize
the upstream `max` reasoning-effort value.

The Ralph queue runs one CodexPotter project per task and passes its command-line arguments to
`codex-potter exec`. The queue's task selection boundary is independent from CodexPotter's
internal round budget, so this change belongs at the `exec` argument boundary rather than in task
selection or round scheduling.

## Goals

- Make `gpt-5.6-luna` the default model for `codex-potter exec`.
- Make `model_reasoning_effort="max"` the default upstream configuration for `exec`.
- Preserve an explicit CLI effort override, such as
  `--config model_reasoning_effort="high"`.
- Allow upstream `max` session metadata to deserialize, display, and load from layered TUI
  configuration.
- Apply the defaults to both human-readable and JSON `exec` modes.
- Preserve the existing behavior of interactive sessions, `resume`, the Ralph queue wrapper, and
  explicitly selected models or effort values.

## Non-goals

- Do not change `.ralph/next_task.sh` or `.ralph/run_potter_queue.sh`.
- Do not change CodexPotter's task selection or internal round scheduling.
- Do not introduce a separate public `--effort` flag; the existing repeatable `--config` mechanism
  remains the source of explicit upstream configuration overrides.
- Do not change the explicit `--xmodel` policy in this task.

## Design

### Exec-only default application

After parsing the top-level CLI arguments and before launching either `run_exec_human` or
`run_exec_json`, apply an `exec`-specific defaulting operation to `UpstreamCodexCliArgs`:

1. If `model` is unset, set it to `gpt-5.6-luna`. An explicit `--model` value remains unchanged.
2. If `config_overrides` does not contain a `model_reasoning_effort` key, append the upstream
   override `model_reasoning_effort="max"`.
3. If one or more CLI config overrides contain `model_reasoning_effort`, preserve them verbatim and
   do not append the default. This lets the last upstream-resolved explicit value, for example
   `high`, continue to control the session.

The key check compares the trimmed key portion before the first `=` so both quoted and unquoted
values work without interpreting or rewriting user input. Other config overrides retain their
existing order and contents.

The resulting `UpstreamCodexCliArgs` is passed through the existing Potter app-server client and
round context unchanged. Its model continues to be carried through `thread/start`/`thread/resume`,
while its reasoning-effort override continues to be forwarded to the upstream `codex app-server`
process as a `--config` argument.

### Reasoning effort model

Add `Max` to `codex_protocol::openai_models::ReasoningEffort`. Existing lowercase serde and display
behavior will encode and render it as `max`.

Extend the TUI layered-config parser to accept `model_reasoning_effort = "max"`. This keeps the
startup/session metadata path consistent with the upstream response path and prevents a valid
`max` value from being rejected as invalid configuration.

### Explicit override precedence

The precedence for `codex-potter exec` is:

1. Explicit CLI `--model` over the `gpt-5.6-luna` default.
2. Explicit CLI `--config model_reasoning_effort=...` over the `max` default.
3. Defaults only fill missing values.

This precedence is limited to the `exec` entry point. Interactive and `resume` invocations retain
their current argument behavior.

## Testing

Add or update focused tests to cover:

- `exec` defaults add `gpt-5.6-luna` and `model_reasoning_effort="max"` when neither value is
  supplied.
- An explicit model remains unchanged while the missing effort receives the default.
- An explicit `model_reasoning_effort="high"` remains unchanged and no `max` override is appended.
- Both quoted and unquoted effort override values are recognized by the key detector.
- `ReasoningEffort::Max` serializes/deserializes as `max`.
- TUI config resolution accepts `model_reasoning_effort = "max"`.
- Existing `to_upstream_codex_args` behavior continues to emit the effective `--config` pair.

Verification will include the focused crate tests, `cargo fmt --check`, and workspace clippy as
appropriate for the final change.

## Error handling

The defaulting operation does not parse user-provided TOML values and therefore does not add new
failure modes. Invalid explicit config values remain handled by the upstream Codex CLI as they are
today. A valid upstream `max` response is accepted by the expanded local enum instead of causing a
deserialization failure.

## Files expected to change

- `cli/src/main.rs` or the adjacent CLI argument flow, for exec-only default application and tests.
- `protocol/src/openai_models.rs`, for `ReasoningEffort::Max`.
- `tui/src/codex_config.rs`, for layered-config parsing and tests.
- Focused documentation or snapshots only where the new default is rendered or described.

