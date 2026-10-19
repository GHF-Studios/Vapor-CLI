# Architecture

## Purpose

Vapor Shell gives authored source a stable command environment without making
that source part of the Steam installation/app root. The executable may be
globally reachable through PATH; process invocation does not determine source
ownership or widen filesystem access.

## Authority hierarchy

1. Vapor manifests are authoritative for Vapor identity, composition, Steam app
   policy, and workflow intent. Application source roots use
   `App-Source.vapor.toml`; installed app roots use `App.vapor.toml`; ordinary
   workspaces use `Workspace.vapor.toml`; content artifacts use role-specific
   filenames.
2. `Cargo.toml` is authoritative for Rust workspaces, packages, targets,
   dependencies, features, and profiles.
3. Canonical filesystem paths are authoritative for containment.
4. `cargo metadata` output is a rebuildable projection of Cargo authority.
5. Prompt context and command output are derived views.

Cargo metadata never creates, renames, or redefines a Vapor entity. Declaration
IDs are inferred from `name` and source-root context; references use fully
qualified IDs.

## Source roots

Vapor accepts two source-root kinds:

- `[root]`: the pure Vapor application source/depot super-repository. It is not a
  Cargo workspace. Direct submodules that declare `[workspace]` and contain
  `Cargo.toml` become routed Cargo workspaces.
- `[workspace]`: a normal Vapor/Cargo source workspace rooted in the same
  directory as its `Cargo.toml`.

Content manifests cannot be standalone source roots. Starting inside
Vapor-CLI, a game, an engine, or another nested artifact escalates to the
highest containing `[root]` or `[workspace]`.

The Steam installation is the app root, not a source root. It is discovered from
the running executable.

## Session startup

```text
current_exe() ── ancestors ──> installed app [root]
                                  │
current_dir()/remembered path ──> external source [root] or [workspace]
                                  │
                    reject if either root overlaps
                                  │
                  compare app-root registration
                                  │
                 validate source Vapor identity
                                  │
                  discover routed Cargo workspaces
                                  │
                    project Cargo metadata with
                  bundled Cargo when it is installed
                                  │
                         enter interactive loop
```

## Service and frontend ownership

The Vapor Client service is the shared application host for Studio and CLI.
Compositions, the Library, content discovery and lifecycle, Workshop operations,
root publishing, authoring mutations, and managed workspace tools are requested
through typed Client protocol actions. The service owns their persistent state,
provider calls, and operation lifecycle; the frontends render snapshots and
submit intent.

The frontends are not yet pure facades. CLI still owns some source discovery and
Cargo metadata projection, runtime-target selection, root local deployment and
package staging, plus OS-level executable handoff and initial Client deployment.
Workspace-tool stdout and stderr now travel as operation-scoped Client events:
CLI renders them to their original terminal streams, while Studio keeps a
bounded output view. Source release staging uses the same event stream for each
Cargo step and its child-process output. The service bounds each line and caps
queued output while a frontend is not polling, preserving lifecycle events and
marking omitted older output. The remaining host/bootstrap boundaries are
genuine local work, not evidence that CLI and Studio have separate content
authorities.

`ShellState` owns the CLI's discovered installation/source roots and its
source-only navigation cursor. The current directory can never become an
installation path. Installation commands print or execute explicit resources
without changing the source cursor.

Cargo integration has three materially different states:

- **invalid structure**: a `[workspace]` source root has no root `Cargo.toml`, or
  a `[root]` child workspace is malformed;
- **unmet prerequisite**: Cargo structure exists, but app-local Cargo is not
  installed or healthy;
- **loaded**: bundled Cargo returned compatible metadata.

The shell retains diagnostics for unmet prerequisites, but script-owned tooling
lifecycle is outside the normal command surface. Cargo-backed workflows
do not proceed until required source structure and tools validate.

## Resolution and validation

`metadata::ResolvedMetadata` is the shared snapshot used by reporting and
action commands. It resolves source-root policy, optional `[root.steam]` policy,
app-root registration state, tool health, and Cargo-derived state once.

Service-owned actions validate against the same workspace and toolchain they
will use, then return a typed operation result. Remaining CLI-owned actions use
`metadata::ResolvedMetadata` and targeted `ValidationPlan`s. For example, root
local deploy and package staging still require CLI orchestration, while a real
root publish additionally requires SteamCMD; `root publish --dry-run` does not.
Commands never install or repair failed prerequisites implicitly.

## Command surface

The interactive shell is a CLI frontend to the shared Client service. Source-
bound actions derive workspace identity from the active source and discovered
app root, then submit typed service requests. The CLI still provides host-level
bootstrap, metadata reporting, local root deployment/package orchestration,
diagnostics, and process handoff.

Scripts may run local content lifecycle operations and dry-run publication
staging or IDE repair, but they may not authenticate Steam, perform real
uploads, create or delete real Workshop authority, or apply project-local IDE
changes.

## Module map

- `app`: startup and REPL control flow.
- `command`: Clap grammar and CLI action adapters; content and publishing state changes are
  dispatched through typed Client actions.
- `cargo_metadata`: optional rebuildable invocation and projection of Cargo's
  JSON output for CLI workspace reporting.
- `discovery`: installation/source discovery and disjoint-root validation.
- `ide`: explicit project-local RustRover/JetBrains status and repair.
- `manifest`: strict Vapor identity vocabulary.
- `metadata`: shared environment resolution, reporting, and targeted preflight.
- source registration and active authoring context are owned by Vapor Core and
  mutated through typed Vapor Client service actions.
- `workspace`: source-root Cargo workspace discovery.
- `workflow`: thin CLI adapters to the service-owned app-local Rust/Cargo
  workflows, plus the remaining direct host bootstrap helpers.
- `app_local_tools`: internal app-local tool readiness model used by metadata and command preflight.
- `state`: active source selection and current content context.
- `prompt`: Reedline presentation adapter.

The executable `main.rs` only reports a fatal startup error and exit status.

## Failure behavior

Fatal workflow failures protect an authority or boundary: missing Vapor or Cargo
manifests, invalid identities, self-targeting, path escape, or overlapping
roots. Missing tools remain inspectable and explicitly repairable; they never
authorize host-tool fallback.
