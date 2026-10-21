# Interactive commands

Run `help` for the command list or `help <COMMAND>` for argument details.
`vapor` without arguments opens the interactive shell. The shell owns source
context and command authority after the app root has been prepared by
Vapor Installer. Host-level direct facades are limited to explicit automation
entrypoints: `source`, `metadata`, `installation`, `binaries`, `libraries`,
`launch`, `content`, `root`, `script run`, `provider`, and `diagnostics`.

Repeatable automation should live in
`resources/vapor/vapor-scripts/NAME.vapor` and run through `vapor script run
NAME`, which executes the same command grammar against a Vapor shell session
state. Use `vapor --startup-script NAME` to enter the interactive shell, run a
source or app-root script, and keep the shell open.
Real Steam uploads and real IDE repair remain manual interactive-shell actions.

Host-level launches may add `--send-diagnostics` to capture the run into the
app-local diagnostics directory. This is private-test tooling, not public
telemetry, and it is off unless the flag is present. The current build stores
small text diagnostics locally and has a future server-upload seam; it does not
ship diagnostics through an arbitrary registry path or require Git for normal
players.

Steam launch examples:

```text
bin/x86_64-unknown-linux-gnu/vapor-entrypoint play --send-diagnostics
bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe play --send-diagnostics
```

## Client entrypoint

### `client launch [launcher|studio|shell|installer] [-- ARGUMENTS...]`

Launch the locally deployed `vapor-entrypoint` using the host platform's
runtime binary directory. This follows the same dispatch path as Steam without
requiring Steam to launch it. The default mode is `studio`; arguments after
`--` are forwarded through the entrypoint to the selected mode:

```text
vapor client launch                         # Studio home
vapor client launch studio -- --workspace /path/to/workspace
vapor client launch launcher                # player-focused Vapor Launcher
vapor client launch play                    # compatibility alias for launcher
vapor client launch shell
vapor client launch installer
```

The installed CLI resolves its app instance from its own location. When
running a source-built CLI from the managed build cache, set `VAPOR_HOME` to the
local app-instance root first. If the entrypoint is missing, run `client deploy
local` from the source workspace.

To launch a specific installed composition, use `composition launch ID`; the
`launcher` mode opens the player interface and does not launch a game by itself.

## Launch

### `launch --current-workspace`

From a directory inside a Vapor Workspace, discover that workspace by walking up
from the current directory and run its default source configuration with Vapor's
managed Rust toolchain. This is the PWD-sensitive source-run path; it inherits the
launching process environment, including a running Steam client when the game
needs one.

### `launch default`

Launch the app manifest's default playable composition through the shared
Vapor Client service. If its default Packagepack has not yet been recorded in
the composition, the service resolves the app's `default` Packagepack seed.
The service owns dependency acquisition, installation state, verification, and
the game handoff. Workshop acquisition uses the signed-in Steam client account.

### `launch selected`

Launch the composition selected in Studio through the same service, without
changing selection. If no playable composition is selected, open Studio and
choose one first.

### `composition list` and `composition <install|verify|repair|update|launch> COMPOSITION_ID`

List playable compositions and their install state, or run a lifecycle operation
on one through the shared Client service. The list marks the active composition.
After the service restarts, installation state is shown as `not checked` until
that composition is verified or used. Run `verify` to refresh it. The service
resolves source and Workshop content, dependencies, installation state, and
launch handoff; the CLI does not implement a second content installer.

```text
vapor composition list
vapor composition install composition-1
vapor composition verify composition-1
vapor composition repair composition-1
vapor composition update composition-1
vapor composition launch composition-1
```

`launch --current-workspace` remains the separate developer path for running a
workspace's default source configuration with the managed Rust toolchain.

## Diagnostics

### `diagnostics status`

Show the app-local diagnostics directory, latest run directory, metadata file,
run log, active capture state, and requested upload transport state.

### `diagnostics upload [--dry-run]`

Preview or send the current/latest local diagnostics run through the configured
diagnostics transport. The current transport boundary is reserved for a future
Vapor HTTP server and intentionally reports not-configured rather than falling
back to Git or a user-managed registry checkout.

Captured runs are stored under:

```text
<app-root>/.vapor/diagnostics/
  runs/YYYY-MM-DD/<unix_timestamp>-<platform>-<short-random>/
    metadata.toml
    vapor.log
  latest.toml
```

## Provider

### `provider git status`

Show the developer Git provider Vapor Shell will use for explicit Git-backed
commands.

### `provider git link PATH`

Persist an explicit Git executable path under app-local state. Vapor validates
the executable with `git --version` before saving it.

### `provider git clear`

Remove the persisted Git executable path. `VAPOR_GIT` and PATH/common OS
discovery may still resolve Git after this.

## Installation resources

### `installation`

Print the Steam installation/app root discovered from the running Vapor
executable.

### `binaries`

Print the app-local binary directory that contains the running Vapor executable.
In release-mode entrypoint launches this is usually `bin/<target>/`.

### `libraries`

Print the app-local `lib` directory when it exists.

## Derived context

### `metadata [--format human|json]`

Resolve the active source root, nearest content, app-root registration state,
app-local tools, root/workspace policy, optional `[root.steam]` policy, and
Cargo metadata into one report. Human-readable output is the default. JSON is
the stable machine interface for scripts and agents.

Metadata reporting is best-effort: missing tools, failed Cargo projection, an
unregistered app root, or absent optional Steam policy are reported as
diagnostics instead of hiding the rest of the environment. Commands use the same
resolved model and reject unmet prerequisites before acting.

## Setup

Normal closed-alpha installation is app-root-tool-owned, not a manual Shell
setup flow. Use the installed app-root tools:

```text
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/status.rs
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/setup_player.rs
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/setup_development.rs
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/teardown_development.rs
rust-script --force <app-root>/resources/vapor/tools/production/app_setup/teardown_player.rs
```

`vapor-installer` is the compiled Steam/app-root launcher for setup flows; it
delegates to the same app-root tool implementation.

`setup_player.rs` prepares player mode: SteamCMD and generated disposable
app-root state. Git is linked by developers through `provider git ...`, not
installed for normal players.
`setup_development.rs` upgrades that app root with Rust/Cargo and cross-build
tooling for developers.

`teardown_development.rs` downgrades developer mode back to player mode without
removing player-mode tooling. `teardown_player.rs` removes script-managed
mutable app-root state; Steam's uninstall feature removes depot-owned
Shell/docs/installer files.

## Cargo workflows

### `fmt|check|test|build [--project PROJECT]`

Run the selected Cargo operation through app-local Rust/Cargo.
`PROJECT` is `all` or a Cargo workspace name discovered from the active source
root. `[workspace]` sources expose their root Cargo workspace. `[root]` sources
expose direct submodules that declare `[workspace]` and contain `Cargo.toml`.

Artifacts go to `output/dev/<project>` inside the app root instead of source
trees. Development tooling should be installed through the app-root
`resources/vapor/tools/production/app_setup/setup_development.rs` tool.
Workspaces that declare `[workspace].binaries` in `Workspace.vapor.toml` can promote
those outputs into `bin/<target>/` through `root build`.

### `clean`

Ask the shared Vapor Client service to remove the active workspace's managed
development build outputs. This clears its app-local `output/dev/<project>`
and user-data development target roots while preserving release outputs,
documentation, installed content, and source files.

### `validate [--project PROJECT]`

For each selected Cargo workspace, run formatting verification, `cargo check`,
tests, strict Clippy, and strict Rustdoc.

## Source session

### `source init basic-content PATH --organization ORG --name NAME [--app-id APPID]`

Create a new source workspace with a basic engine, game, and packagepack. The
target path must be empty or absent. The generated workspace includes the
managed toolchain pin and Cargo-root `[[project]]` entries. Its nested
`[workspace].projects` content registry includes all three roles, while only
the code-bearing Engine and Game are Cargo packages; Packagepack is metadata
only.

If `--app-id` is omitted, Vapor uses the installed app's `[root.steam].app-id`.
After creation, Vapor opens the new source root.

First local proof:

```text
content validate
content deploy ORG/NAME/NAME-packagepack --select
```

First Workshop publication should create dependencies before the packagepack:

```text
content create ORG/NAME/NAME-engine --account ACCOUNT --yes
source repair --write
content create ORG/NAME/NAME-game --account ACCOUNT --yes
source repair --write
content create ORG/NAME/NAME-packagepack --account ACCOUNT --yes
```

### `source status`

Report whether a source is open, the active source identity and cursor, and the
number of indexed sources.

### `source open SOURCE`

Open a Vapor source root by registered name or path. A path is resolved,
validated, added to the app-local source registry, and persisted as the active
source for later shell launches.

The active source must be outside the installed app root. Once opened,
source-backed workflows are confined to that source root.

### `source close`

Close the active source and return the shell to its app-only state. Setup,
source-registry, metadata, and installation-inspection commands remain
available; source-backed Cargo/content workflows wait for another
`source open`.

### `source list`

List source roots registered under the current app root.

### `source add [PATH]`

Validate and register a source root. `PATH` defaults to the process directory
used to start Vapor. Registration does not open the source; use
`source open NAME` or `source open PATH` for that.

### `source remove SOURCE`

Remove a registered source by local name or fully qualified identity. If the
removed source is active, run `source close` or `source open` another source
before source workflows.

### `source sync`

Reserved for controlled source-provider synchronization. The current
implementation reports the active source and explains that no synchronization
is applied yet.

### `source repair [--write]`

Inspect source registry state and safe source metadata repairs. Without
`--write`, this is read-only. With `--write`, Vapor updates dependency
`workshop-id` fields it can derive from sibling content artifacts'
`published-file-id` values.

This is meant for the first Workshop publication loop: after creating an engine
or game item, run `source repair --write` so packagepack/game dependency
metadata carries the newly assigned Workshop IDs before dependent items are
created or published.

## Documentation

### `docs build|publish|path|open [TOPIC]`

Build Rustdoc for discovered Cargo workspaces into the installed `docs/` tree,
publish the curated Vapor documentation site to the configured Vapor Docs
server, print a generated document path, or open it asynchronously. Publishing
uses `VAPOR_DOCS_ADMIN_TOKEN` from the Client service environment and the
`VAPOR_DOCS_BASE_URL` setting (default `https://vapor.ghf-studios.site/docs`).
It is an explicit operation and is not part of local deployment.

## IDE setup

### `ide status`

Inspect project-local RustRover/JetBrains settings for the active source root.
This is read-only. It reports the selected source root, `.idea` directory,
app-local Rust/Cargo bin directory, Rust standard-library source
status, and the state of the files Vapor manages.

### `ide repair [--dry-run]`

Write project-local IDE settings for the active source root so RustRover can
see app-local Rust/Cargo and routed Cargo workspaces.

The current first pass manages only files under the selected source root's
`.idea` directory:

- `.idea/cargoProjects.xml` for routed Cargo workspace manifests;
- `.idea/rust.xml` for the Rust/Cargo path and stdlib source path when
  packaged;
- `.idea/vapor.xml` for Vapor-owned app-root, Cargo home, rustup home, Cargo,
  rustc, rustup, Git, and source identity metadata.

`--dry-run` previews which project-local files would be written without
changing them. Real IDE repair must be typed manually in the interactive shell;
scripts may run `ide status` and `ide repair --dry-run`, but not real
`ide repair`.

## Root application/depot workflows

### `root build [--release] [--target TARGET]... [--release-targets] [--host-only]`

Build installable Cargo workspaces into app-root build outputs under
`output/dev/<project>/`, or `output/release/<project>/` with `--release`.
Installable means a workspace declares at least one
`[workspace].binaries` entry. When `[root.runtime].targets` is declared,
omitting target flags builds that full runtime matrix by default. Repeat
`--target` to build an explicit custom subset, such as only Windows GNU/LLVM.
`--release` uses Cargo's optimized profile and does not promote outputs.

`root build` does not refresh installed app binaries, docs, Vapor scripts, or
launch scripts. Use `root deploy` to promote already-built app outputs into the
local Steam/app root.

`--release-targets` is an explicit spelling of the manifest target-matrix
default; it does not select Cargo's release profile. Use `--host-only` for a
local smoke pass that builds only Cargo's host target. Do not combine `--target`,
`--release-targets`, and `--host-only`.

### `root deploy [--skip-docs] [--target TARGET]... [--release-targets] [--host-only]`

Promote already-built application binaries from `[workspace].binaries` into the
local Steam installation/app root under `bin/<target>/`, then refresh installed
docs, Vapor scripts, setup tools, and platform launch scripts. This is a local
app-root deploy/sync operation; it does not upload to Steam.

When `[root.runtime].targets` is declared, omitting target flags deploys that
full runtime matrix by default. Repeat `--target` to deploy an explicit custom
subset. Use `--host-only` for a local host-only deploy. Use `--skip-docs` for a
faster binary/script refresh.

### `root package [--target TARGET]... [--release-targets] [--host-only]`

Build and refresh the local Steam app root using Cargo's optimized release
profile, assemble the clean allowlisted
split-depot app payload, and smoke-check the staged package without invoking
SteamCMD. The default root payload is runtime-only: the common depot carries
`App.vapor.toml`, `docs/`, app scripts, and packaged examples; platform depots
carry selected `bin/<target>/` application binaries, including
`vapor-entrypoint[.exe]`, and target-matching `bin/vapor-launch.*` scripts.

When `[root.runtime].targets` is declared, omitting target flags stages that
full matrix by default. Repeat `--target` to build and stage a deliberate
custom subset. Use `--host-only` for a local host-only package.

### `root publish [--account ACCOUNT] [--branch BRANCH] [--dry-run] [--yes]`

Validate, build, promote binaries, build docs, stage the clean split-depot app
payload, smoke-check it, generate SteamPipe app/depot VDFs, and optionally
upload it. The default upload payload is runtime-only; Loo-Cast and other content
artifacts are published through `content publish`, not through the app depot.

Real publication always uses the complete `[root.runtime].targets` matrix and
runs validation/build/promotion before staging. Narrow target selection,
`--host-only`, and `--skip-build` are dry-run/local-package escape hatches only;
they are rejected for real uploads. The depot smoke check rejects staged
platform depots when their matching `bin/vapor-launch.*` script,
`bin/<target>/vapor-entrypoint[.exe]`, `bin/<target>/vapor[.exe]`,
`bin/<target>/vapor-installer[.exe]`, `bin/<target>/vapor-gui[.exe]`,
`bin/<target>/vapor-ide-host[.exe]`, or required Windows runtime DLL payload
is missing. Launchable binaries are declared by each direct root workspace's
`[workspace].binaries` list and promoted into the app root before staging.
Real publication preflight requires app-local Rust/Cargo, cross-build tooling,
and SteamCMD. Explicit Git-backed operations use the linked developer Git
provider instead of app-local Git.

`--dry-run` writes the staged payload and preview VDF without requiring
SteamCMD or performing an upload. A real upload requires `--account ACCOUNT`
and `--yes`, and must be typed manually in the interactive shell. The branch
defaults to `[root.steam].development-branch` and must be non-default.

## Content workflows

### `content status`

Report the nearest typed content node under the source cursor and print the
app-root content layout used for installed artifact roots, cache, and generated
state. Packagepacks, Enginepacks, Gamepacks, and Modpacks are content
artifacts, not application depot roots.

### `content list`

Refresh the shared Vapor Client catalog and show authored workspace content,
downloaded Library items, and installed content from the same read model. A
child directory is source content only when the active workspace registers it
under `[[workspace.projects]]` and the child role manifest declares a content
identity. Incomplete or failed catalog refreshes are reported as errors instead
of being shown as an empty Library.

### `content validate [ARTIFACT]`

Validate source content metadata, required composition/dependency references,
conflicts, and Workshop publication intent through the shared service. Omit
`ARTIFACT` to validate every registered source artifact.

### `content build [--target TARGET]... [--release-targets] [--host-only]`

Build the active content workspace through app-local Cargo. This uses the same
development tooling preflight as other Cargo workflows and writes build output
under the app root. When `[workspace.runtime].targets` is declared, omitting
target flags builds that full matrix by default. Use `--target` to build an
explicit custom subset such as only `x86_64-pc-windows-gnullvm`. Use
`--host-only` for a local host build.

### `content deploy ARTIFACT [--select] [--target TARGET]... [--release-targets] [--host-only]`

Install the selected source artifact into the app-owned installed-content tree
through Vapor Client. The service builds the selected artifact and its required
source dependencies as part of installation. Declared `binaries` and
`libraries` are copied from app-local Cargo output into each deployed artifact root under
`bin/<target>/` and `lib/<target>/`. This is local-only: it does not create,
publish, delete, subscribe to, or upload Workshop items.

Use `--select` when deploying a packagepack that should become the active
playable packagepack. When `[workspace.runtime].targets` is declared, omitting
target flags deploys the full matrix. Use `--host-only` for quick local
iteration on the current machine.

### `content package ARTIFACT [--target TARGET]... [--release-targets] [--host-only] [--dry-run]`

Build the complete source workspace through the managed toolchain, then stage
one deployable artifact root under `output/content/packages/` through the
shared Client service. Vapor writes a resolved deployed role manifest,
fingerprints the staged root, and records a receipt. Declared `binaries` and
`libraries` are copied from app-local Cargo output into `bin/<target>/` and
`lib/<target>/`; the deployed manifest records their staged filenames in
target-specific `runtime` entries. When
`[workspace.runtime].targets` is declared, omitting target flags stages that
full matrix into one package root. Repeat `--target` for an intentional custom
subset, or use `--host-only` for a local host-only package.

Release Workshop packages should be single logical artifact roots that contain
every shipped runtime target, for example Linux and Windows GNU/LLVM side by side.
Do not create separate Workshop items, app roots, or publication branches just
to split operating systems. Runtime selection happens when Vapor installs or
launches content by choosing the matching `bin/<target>/` and `lib/<target>/`
payload.

`--dry-run` still builds the workspace and computes the intended artifact path
and deployed-artifact fingerprint without writing package files.

### `content acquire ARTIFACT_OR_WORKSHOP_ID`

Acquire a local authored artifact into the Library cache, or acquire a numeric
Steam Workshop item through the shared Vapor Client service and Steamworks.

### `content subscribe PUBLISHED_FILE_ID`

Subscribe to the Steam Workshop item and request it for the Library through the
shared service. Steam subscription state remains distinct from local Library
availability and composition membership.

### `content download ARTIFACT_OR_WORKSHOP_ID...`

Download Workshop PublishedFileIds through the shared service. Cache authored
local artifacts through its typed Library operation; keep authored selectors
and Workshop IDs in separate commands.

### `content install ARTIFACT_OR_WORKSHOP_ID [--account ACCOUNT]`

Run installation through the shared Client service. When this shell has a source
workspace open, it is passed as operation context; otherwise install resolves
from Vapor's cache and provider sources. Required dependencies are installed
with the target.

If the selector matches a root content seed or numeric PublishedFileId and no
cache exists, Vapor downloads it through SteamCMD before installing. Use
`--account ACCOUNT` for private/unreleased Workshop access.

### `content update [ARTIFACT_OR_WORKSHOP_ID]`

Update one installed item, or every installed item when omitted, through the
shared Client service, using the current source workspace when available.

### `content verify [ARTIFACT_OR_WORKSHOP_ID]`

Verify installed artifacts through the shared Client service. Omit the target
to verify everything in the installed-content index.

### `content selected`

Print the Packagepack root of the currently selected playable composition.
This reads the same persistent composition state used by Studio and Play.

### `content select ARTIFACT_OR_WORKSHOP_ID`

Set the active composition's Packagepack root to an installed Packagepack.
Studio and `launch selected` use this same persistent composition choice.

### `content deselect`

Clear the active composition's Packagepack root.

### `content repair [ARTIFACT_OR_WORKSHOP_ID]`

Verify installed content, quarantine corrupted artifact roots under
`content/quarantine/`, and reinstall from the current source workspace or
Vapor's cache when available. Omit the target to repair all installed items.

### `content disable ARTIFACT_OR_WORKSHOP_ID`

Move installed content to `content/disabled/` and update the content index
without deleting its artifact root.

### `content enable ARTIFACT_OR_WORKSHOP_ID`

Move disabled content back to `content/installed/` and update the content
index.

### `content uninstall ARTIFACT_OR_WORKSHOP_ID`

Remove installed or disabled artifact roots through the shared Client service.
Direct mutations that would invalidate a composition's resolved graph are
rejected; use that composition's install, update, or repair workflow instead.

### `content create ARTIFACT [--target TARGET]... [--release-targets] [--host-only] [--dry-run] [--account ACCOUNT] [--yes]`

Stage a service-owned Workshop review for a new item, or publish that reviewed
package after explicit confirmation. A dry-run shows the package identity,
target matrix, fingerprint, files, and dependencies without uploading. Real
item creation is a Steam authority-changing action and must be typed manually
in the interactive shell. When `[workspace.runtime].targets` is declared,
creation uses that matrix by default. Real creation rejects `--target` and
`--host-only` and requires the declared matrix to contain Linux and Windows
targets. It validates and builds that matrix before packaging and upload. Real
creation preflight requires app-local Rust/Cargo, cross-build tooling, and
SteamCMD. Repeat `--target` or use `--host-only` only for dry-run previews.

### `content publish ARTIFACT... [--target TARGET]... [--release-targets] [--host-only] [--dry-run] [--account ACCOUNT] [--change-note TEXT] [--yes]`

Stage a service-owned review for one artifact or preview a multi-artifact
provider batch. A single-artifact dry-run shows the package identity, target
matrix, fingerprint, files, and dependencies without uploading. Multi-artifact
publication remains one provider batch transaction. A real upload requires
`--account ACCOUNT`, `--yes`, existing PublishedFileIds in the artifacts' role
manifests, and must be typed manually in the interactive shell. When
`[workspace.runtime].targets` is declared,
publishing packages that matrix by default. Real publication rejects `--target`
and `--host-only` and requires the declared matrix to contain Linux and Windows
targets. It validates and builds that matrix before packaging and upload.
Real publication preflight requires app-local Rust/Cargo, cross-build tooling,
and SteamCMD. An artifact ID may omit its version only when it identifies one
source project in the open workspace; use `ID@VERSION` when the ID has multiple
versions. Repeat `--target` or use `--host-only` only for dry-run previews.

The intended release path is plain `content publish ...`: one Workshop item
update per artifact, with all supported platform binaries and libraries inside
that item. Steam Workshop beta-branch versioning is reserved for app-branch
compatibility ranges, not for Linux-vs-Windows payload splitting.

### `content delete ARTIFACT_OR_WORKSHOP_ID --dry-run`

Record a safe preview of deleting or retiring a Workshop item. Real deletion is
a SteamUGC authority-changing action and is refused unless a controlled
SteamUGC provider implements it.

## Scripts

### `script run NAME [--dry-run]`

Read `resources/vapor/vapor-scripts/NAME.vapor` and execute each non-comment
line through this same command parser. Source scripts are preferred when a
source is open; app-root scripts under the installed app's
`resources/vapor/vapor-scripts/` are used as a fallback.
`--dry-run` prints the commands without executing them.

Scripts stop on error and cannot recursively invoke scripts, exit the host REPL,
perform real publishes, delete Workshop items, send diagnostics, or apply IDE
repairs. Scripts may run status inspection and Workshop download/install
operations, including account-backed SteamCMD acquisition when a private or
unreleased item requires visible Steam authentication.

## Session control

### `exit`

Exit the shell. `quit` is an alias. Ctrl-D also exits; Ctrl-C cancels the
current input line.
