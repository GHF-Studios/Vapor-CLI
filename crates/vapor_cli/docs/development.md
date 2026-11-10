# Development guide

## Repository layout

```text
crates/vapor_cli/
├── README.md
├── docs/
├── scripts/
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── app.rs
│   ├── command.rs
│   ├── cargo_metadata.rs
│   ├── distribution.rs
│   ├── documentation.rs
│   ├── discovery.rs
│   ├── ide.rs
│   ├── manifest.rs
│   ├── metadata/
│   │   ├── mod.rs
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   └── render.rs
│   │   └── validation.rs
│   ├── prompt.rs
│   ├── steam.rs
│   ├── terminal.rs
│   ├── app_local_tools.rs
│   ├── workflow.rs
│   ├── workspace.rs
│   └── state.rs
├── templates/
│   └── ide/
└── tests/
    ├── common/
    ├── samples/
    ├── cargo_metadata.rs
    ├── command.rs
    ├── discovery.rs
    ├── ide.rs
    ├── installation_commands.rs
    ├── manifest.rs
    ├── metadata.rs
    ├── state.rs
    ├── workflow.rs
    └── workspace.rs
```

`lib.rs` is the documented implementation surface. `main.rs` remains a thin
process adapter. Behavioral tests live outside `src` and exercise public
contracts as downstream code would.

## Running locally

Direct `cargo run` places the executable under the source repository, which is
not the product surface. Build and verification work should use Vapor-managed
Rust/Cargo tooling in Vapor home. The independent Vapor home owns release-mode
launches through `bin/<target>/vapor-entrypoint[.exe]` and the other runtime
applications. Steam depots contain only the Loo-Cast composition manifest and
its three products; they are not a development install target.

After the installed binary works, run `source open /path/to/source` from the
installed shell. This validates and registers the external source selection
without moving source into Vapor home.

## Documentation policy

The crate denies missing public documentation and forbids unsafe code. Public
items should include whichever sections clarify their contract:

- purpose and authority;
- examples for non-obvious construction or parsing;
- `# Errors` for fallible functions;
- `# Panics` only when unavoidable;
- security or boundary invariants;
- relationship to app-owned versus critical source state.

Long-form concepts belong in `docs/`; API-specific contracts stay beside code.
README links provide the entry path instead of duplicating every detail.

Design checkpoints define vocabulary and product intent. User-facing docs and
command references must distinguish implemented behavior from planned behavior
instead of presenting design goals as already shipped commands.

## Adding a command

1. Add a documented `ShellCommand` variant.
2. Use a Clap `ValueEnum` for static finite argument domains.
3. Describe unrestricted paths or numeric domains with semantic value names.
4. Implement the effect in `command::execute`. Reuse `ResolvedMetadata` and a
   targeted `ValidationPlan` when the command depends on environment state.
5. Decide explicitly whether it reads source, reads installation state, or
   mutates source state. Installation navigation is not allowed implicitly.
6. If the command mutates source, installation, IDE settings, Steam state, or
   publication state, decide whether it needs status and `--dry-run` preview
   support before implementation is considered complete.
7. Add integration coverage in `tests/command.rs` or a focused new file.
8. Update `docs/commands.md`.

## Adding a manifest identity

1. Extend `ContentKind` or add a new source-root/project identity deliberately.
2. Add the deserialization field and mapping in `manifest.rs`.
3. Add exhaustive integration coverage in `tests/manifest.rs`.
4. Document syntax, semantics, and composition role in `docs/manifests.md`.
5. Update shared Vapor vocabulary rather than introducing a shell-only spelling.

## Adding a workspace package

1. Add the Cargo package to its containing Cargo workspace.
2. If it is Vapor content, add the matching role-specific manifest such as
   `Engine.vapor.toml`, `Game.vapor.toml`, `Engine-Mod.vapor.toml`,
   `Game-Mod.vapor.toml`, `Extension-Mod.vapor.toml`, or
   `Packagepack.vapor.toml`.
3. Do not add a Vapor manifest for an ordinary non-content Cargo package.
4. Do not add declaration-side `id`; references use full IDs, declarations infer
   them.
5. For Vapor-Root app membership, add or update a direct Git submodule that is a
   `[workspace]` repository with its own `Workspace.vapor.toml` and
   `Cargo.toml`.
6. Extend workspace, Cargo-metadata, and workflow integration tests where the
   package affects Vapor behavior.

## Changing discovery

Discovery changes require tests for both roots, overlap rejection, canonical
containment, behavior below nested content, and escalation from the shell
component to its containing `[root]`. Do not introduce a fallback that
places authored source inside installation state or permits self-targeting.

## Validation

Run tests, strict Clippy, rustdoc with warnings denied, and formatting before
handoff. Cargo metadata tests use a fake bundled Cargo executable and do not
depend on the developer's global Cargo installation. Tests must distinguish an
invalid missing Cargo manifest from a repairable missing bundled Cargo tool.
