# `exec` Default Model and Reasoning Effort Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `codex-potter exec` default to `gpt-5.6-luna` with `model_reasoning_effort="max"` while preserving explicit CLI effort overrides such as `high`.

**Architecture:** Add an exec-only defaulting method to `UpstreamCodexCliArgs`, invoke it after CLI parsing and before either human or JSON exec mode launches, and leave interactive/resume argument handling unchanged. Extend the shared reasoning-effort enum and TUI layered-config parser so `max` survives upstream response deserialization and local metadata rendering.

**Tech Stack:** Rust 2024, Cargo workspace, Clap argument parsing, Serde JSON/TOML, `pretty_assertions`, `cargo test`, `cargo fmt`, and `cargo clippy`.

## Global Constraints

- Defaults apply only to `codex-potter exec`.
- Explicit CLI `--model` takes precedence over the `gpt-5.6-luna` model default.
- Explicit CLI `--config model_reasoning_effort=...` takes precedence over the `max` effort default.
- The existing repeatable `--config` mechanism remains the only explicit effort override API.
- Do not change `.ralph/next_task.sh`, `.ralph/run_potter_queue.sh`, task selection, or internal round scheduling.
- Do not change the explicit `--xmodel` policy in this task.
- Preserve unrelated working-tree changes and never discard user changes.
- Use `pretty_assertions::assert_eq` and whole-value comparisons in tests where practical.
- Run `cargo fmt` and `cargo clippy` before claiming completion.

---

### Task 1: Add the `max` reasoning-effort value

**Files:**
- Modify: `protocol/src/openai_models.rs`
- Modify: `tui/src/codex_config.rs`
- Test: Inline unit tests in the two files above

**Interfaces:**
- Consumes: Existing `ReasoningEffort` serde/display derives and TUI `parse_reasoning_effort`.
- Produces: `ReasoningEffort::Max`, serialized as the lowercase string `"max"`, and TUI config resolution for `model_reasoning_effort = "max"`.

- [ ] **Step 1: Write the failing protocol regression test**

Add a `#[cfg(test)]` module to `protocol/src/openai_models.rs` with this test:

```rust
#[cfg(test)]
mod tests {
    use super::ReasoningEffort;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn max_reasoning_effort_round_trips_as_lowercase_max() {
        assert_eq!(serde_json::to_value(ReasoningEffort::Max), Ok(json!("max")));
        assert_eq!(
            serde_json::from_value::<ReasoningEffort>(json!("max")),
            Ok(ReasoningEffort::Max)
        );
    }
}
```

- [ ] **Step 2: Run the protocol test and verify the expected failure**

Run:

```bash
cargo test -p codex-protocol max_reasoning_effort_round_trips_as_lowercase_max
```

Expected: compilation fails because `ReasoningEffort::Max` does not yet exist. Do not change the
test to make it compile before adding the production enum variant.

- [ ] **Step 3: Add the enum variant**

In `protocol/src/openai_models.rs`, add `Max` after `XHigh` in `ReasoningEffort`:

```rust
    XHigh,
    Max,
```

The existing `#[serde(rename_all = "lowercase")]`, `#[strum(serialize_all = "lowercase")]`, and
derived `Display` implementation must remain unchanged so the new value serializes and displays as
`max`.

- [ ] **Step 4: Run the protocol test and verify it passes**

Run the same focused command. Expected: one passing test and no test failures.

- [ ] **Step 5: Write the failing TUI config regression test**

Add this test beside the existing layered-config tests in `tui/src/codex_config.rs`:

```rust
#[test]
#[serial]
fn resolves_max_reasoning_effort_from_user_config() {
    let codex_home = tempfile::tempdir().expect("codex home");
    let _env = EnvVarGuard::set("CODEX_HOME", codex_home.path());
    write_config(
        &codex_home.path().join(CONFIG_TOML_FILENAME),
        r#"
model = "gpt-5.6-luna"
model_reasoning_effort = "max"
"#,
    );

    let cwd = tempfile::tempdir().expect("cwd");
    let resolved = resolve_codex_model_config(cwd.path()).expect("resolve config");

    assert_eq!(
        resolved,
        ResolvedCodexModelConfig {
            model: "gpt-5.6-luna".to_string(),
            reasoning_effort: Some(ReasoningEffort::Max),
            is_fast: false,
        }
    );
}
```

- [ ] **Step 6: Run the TUI test and verify the expected failure**

Run:

```bash
cargo test -p codex-tui resolves_max_reasoning_effort_from_user_config
```

Expected: the test fails with the existing invalid-effort configuration error for `max`.

- [ ] **Step 7: Extend the TUI parser minimally**

In `tui/src/codex_config.rs`, extend `parse_reasoning_effort` with the `max` arm:

```rust
        "xhigh" => Some(ReasoningEffort::XHigh),
        "max" => Some(ReasoningEffort::Max),
```

- [ ] **Step 8: Run both focused tests**

Run:

```bash
cargo test -p codex-protocol max_reasoning_effort_round_trips_as_lowercase_max
cargo test -p codex-tui resolves_max_reasoning_effort_from_user_config
```

Expected: both tests pass.

- [ ] **Step 9: Commit the reasoning-effort support**

```bash
git add protocol/src/openai_models.rs tui/src/codex_config.rs
git commit -m "feat: support max reasoning effort"
```

### Task 2: Add exec defaulting and preserve explicit effort arguments

**Files:**
- Modify: `cli/src/app_server/upstream_cli_args.rs`
- Test: Unit tests in `cli/src/app_server/upstream_cli_args.rs`

**Interfaces:**
- Consumes: `UpstreamCodexCliArgs { model, config_overrides, ... }`.
- Produces: `pub fn apply_exec_defaults(&mut self)` that fills missing values with `gpt-5.6-luna` and `model_reasoning_effort="max"` without rewriting explicit values.

- [ ] **Step 1: Write failing tests for default and explicit-override behavior**

Add these tests to the existing `#[cfg(test)] mod tests` in `cli/src/app_server/upstream_cli_args.rs`:

```rust
    #[test]
    fn exec_defaults_fill_model_and_reasoning_effort() {
        let mut args = UpstreamCodexCliArgs::default();

        args.apply_exec_defaults();

        assert_eq!(args.model.as_deref(), Some("gpt-5.6-luna"));
        assert_eq!(
            args.config_overrides,
            vec!["model_reasoning_effort=\"max\"".to_string()]
        );
    }

    #[test]
    fn exec_defaults_preserve_explicit_model_and_high_effort() {
        let mut args = UpstreamCodexCliArgs {
            model: Some("custom-model".to_string()),
            config_overrides: vec!["model_reasoning_effort=\"high\"".to_string()],
            ..Default::default()
        };

        args.apply_exec_defaults();

        assert_eq!(
            args,
            UpstreamCodexCliArgs {
                model: Some("custom-model".to_string()),
                config_overrides: vec!["model_reasoning_effort=\"high\"".to_string()],
                ..Default::default()
            }
        );
    }

    #[test]
    fn exec_defaults_detect_effort_key_with_whitespace_and_unquoted_value() {
        let mut args = UpstreamCodexCliArgs {
            config_overrides: vec!["  model_reasoning_effort = high".to_string()],
            ..Default::default()
        };

        args.apply_exec_defaults();

        assert_eq!(
            args.config_overrides,
            vec!["  model_reasoning_effort = high".to_string()]
        );
    }
```

- [ ] **Step 2: Run the new tests and verify the expected failure**

Run:

```bash
cargo test -p codex-potter-cli exec_defaults_
```

Expected: compilation fails because `apply_exec_defaults` is not defined.

- [ ] **Step 3: Add constants and the defaulting method**

In `cli/src/app_server/upstream_cli_args.rs`, add the constants near the argument type and add this
documented method inside `impl UpstreamCodexCliArgs`:

```rust
const EXEC_DEFAULT_MODEL: &str = "gpt-5.6-luna";
const EXEC_DEFAULT_REASONING_EFFORT: &str = "max";

impl UpstreamCodexCliArgs {
    /// Fill defaults used by `codex-potter exec` without rewriting explicit CLI values.
    pub fn apply_exec_defaults(&mut self) {
        if self.model.is_none() {
            self.model = Some(EXEC_DEFAULT_MODEL.to_string());
        }

        let has_reasoning_effort_override = self.config_overrides.iter().any(|override_kv| {
            override_kv
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "model_reasoning_effort")
        });
        if !has_reasoning_effort_override {
            self.config_overrides.push(format!(
                "model_reasoning_effort=\"{EXEC_DEFAULT_REASONING_EFFORT}\""
            ));
        }
    }
}
```

Keep the existing `to_upstream_codex_args` implementation unchanged; the method must store the
default in `config_overrides` so the existing forwarding code emits it as an upstream `--config`
pair. Do not parse or rewrite the value of an explicit override.

- [ ] **Step 4: Run the focused argument tests**

Run:

```bash
cargo test -p codex-potter-cli exec_defaults_
cargo test -p codex-potter-cli upstream_args_translate_profile_and_search_to_config_overrides
```

Expected: all selected tests pass, including the pre-existing forwarding test.

- [ ] **Step 5: Commit the defaulting helper**

```bash
git add cli/src/app_server/upstream_cli_args.rs
git commit -m "feat: add exec model and effort defaults"
```

### Task 3: Wire defaults into `exec`, document the behavior, and verify the workspace

**Files:**
- Modify: `cli/src/main.rs`
- Modify: `docs/wiki/cli.md`
- Test: Unit tests in `cli/src/main.rs`

**Interfaces:**
- Consumes: `UpstreamCodexCliArgs::apply_exec_defaults(&mut self)` from Task 2.
- Produces: Both `run_exec_human` and `run_exec_json` receive the same effective args, while the
  non-`exec` path continues to receive the original parsed args.

- [ ] **Step 1: Write a failing wiring test**

Add this test to `cli/src/main.rs`'s existing test module before implementing the helper:

```rust
    #[test]
    fn exec_argument_preparation_adds_luna_max_defaults() {
        let prepared = prepare_exec_upstream_cli_args(Default::default());

        assert_eq!(prepared.model.as_deref(), Some("gpt-5.6-luna"));
        assert_eq!(
            prepared.config_overrides,
            vec!["model_reasoning_effort=\"max\"".to_string()]
        );
    }
```

- [ ] **Step 2: Run the wiring test and verify the expected failure**

Run:

```bash
cargo test -p codex-potter-cli exec_argument_preparation_adds_luna_max_defaults
```

Expected: compilation fails because `prepare_exec_upstream_cli_args` is not defined.

- [ ] **Step 3: Implement the helper and use it only in the exec branch**

Near the existing exec verbosity helpers, implement this helper:

```rust
fn prepare_exec_upstream_cli_args(
    mut upstream_cli_args: crate::app_server::UpstreamCodexCliArgs,
) -> crate::app_server::UpstreamCodexCliArgs {
    upstream_cli_args.apply_exec_defaults();
    upstream_cli_args
}
```

In `main`, inside the `if let Some(CliCommand::Exec
{ ... })` branch and before choosing human vs JSON mode, add:

```rust
        let exec_upstream_cli_args = prepare_exec_upstream_cli_args(upstream_cli_args.clone());
```

Pass `exec_upstream_cli_args` into `ExecRunConfig` in both `run_exec_json` and `run_exec_human`
branches. Leave the later app-server, interactive, and resume code using the original
`upstream_cli_args` clone so the default remains exec-only.

- [ ] **Step 4: Add explicit-precedence coverage to the wiring test module**

Add this test in `cli/src/main.rs`:

```rust
    #[test]
    fn exec_argument_preparation_preserves_explicit_high_effort() {
        let prepared = prepare_exec_upstream_cli_args(crate::app_server::UpstreamCodexCliArgs {
            config_overrides: vec!["model_reasoning_effort=\"high\"".to_string()],
            ..Default::default()
        });

        assert_eq!(prepared.model.as_deref(), Some("gpt-5.6-luna"));
        assert_eq!(
            prepared.config_overrides,
            vec!["model_reasoning_effort=\"high\"".to_string()]
        );
    }
```

Also verify that the defaulting helper is opt-in and does not mutate the argument shape used by
non-`exec` paths:

```rust
    #[test]
    fn unprepared_upstream_arguments_have_no_exec_defaults() {
        let args = crate::app_server::UpstreamCodexCliArgs::default();

        assert_eq!(args.model, None);
        assert!(args.config_overrides.is_empty());
    }
```

- [ ] **Step 5: Run the wiring tests**

Run:

```bash
cargo test -p codex-potter-cli exec_argument_preparation_
```

Expected: both wiring tests pass.

- [ ] **Step 6: Document the exec defaults**

In `docs/wiki/cli.md`, add an `exec` option note next to the existing `exec` behavior:

```markdown
- When no `--model` is supplied, `exec` uses `gpt-5.6-luna` with
  `model_reasoning_effort="max"` by default.
- An explicit `--config model_reasoning_effort="high"` (or another effort value) overrides the
  default for that invocation.
```

Do not change the Ralph scripts or the interactive/resume documentation.

- [ ] **Step 7: Run focused crate tests**

Run:

```bash
cargo test -p codex-protocol
cargo test -p codex-tui
cargo test -p codex-potter-cli
```

Expected: all tests in all three crates pass.

- [ ] **Step 8: Run formatting, lint, and diff checks**

Run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
git diff --check
git status --short
```

Expected: formatting makes no uncommitted changes afterward, Clippy exits successfully, diff check
reports no whitespace errors, and status lists only the intended implementation/documentation
files or the task's commits.

- [ ] **Step 9: Commit the exec wiring and documentation**

```bash
git add cli/src/main.rs docs/wiki/cli.md
git commit -m "feat: default exec to gpt-5.6-luna max"
```
