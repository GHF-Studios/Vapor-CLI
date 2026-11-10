# Vapor Shell

Vapor Shell is the interactive boundary between the independently installed
Vapor home, a Steam-owned product root, and an external source root containing
critical authored source.

At runtime, the shell works with two active filesystem roots:

```text
Vapor home                     Product root             Active source root
├── Vapor-Home.toml            ├── App.vapor.toml        ├── Workspace.vapor.toml
├── bin/<target>/runtime       ├── content shipped by    │   or App-Source.vapor.toml
├── config/                    │   the product owner     ├── Cargo.toml
├── content/                   └── local product files   └── authored source
├── data/state/
├── docs/, examples/, resources/
├── sources/
├── cache/build/<target>/
└── tools/{cargo,rustup}/
```

The roots must not overlap. Vapor discovers its installation from the running
executable or `VAPOR_HOME`; selected product context is supplied through
`VAPOR_PRODUCT_ROOT`. Source roots are opened explicitly by path or remembered
name, and the last active source is stored under Vapor home. Interactive
filesystem navigation is confined to the opened source root.

The product model also distinguishes Vapor application source such as this
repository. It is not the installed Vapor home or the installed product root.

## Documentation

- [Architecture](docs/architecture.md): authority, state ownership, module map,
  and failure behavior.
- [Discovery and boundaries](docs/discovery.md): both discovery algorithms,
  expected installation layout, canonicalization, and overlap rejection.
- [Vapor manifests](docs/manifests.md): root, workspace, project, content,
  composition, trait, and slot syntax.
- [Content lifecycle](docs/content.md): Workshop-backed package, cache,
  install, verify, repair, uninstall, and publication-preview workflows.
- [Commands](docs/commands.md): command behavior, arguments, and which root each
  command can affect.
- [Cargo integration](docs/cargo-metadata.md): required Rust workspaces,
  authority boundaries, nested-workspace consequences, and derived metadata.
- [Setup](docs/setup.md): standalone Vapor installation and Player/Developer
  roles.
- [Distribution](docs/distribution.md): allowlisted staging, exclusions, docs,
  launch entrypoints/scripts, SteamPipe templates, and smoke validation.
- [Steam development](docs/steam-development.md): root publish previews, manual
  upload confirmation, beta publishing, and persistent cache state.
- [Command scripts](docs/scripts.md): reusable REPL command sequences exposed
  through the script CLI facade.
- [Development](docs/development.md): source layout, extension checklists, tests,
  and documentation policy.
- [Design checkpoints](docs/design/README.md): owner-reviewed direction that is
  authoritative only where it matches implemented and verified behavior.

## Core guarantees

- Authored source never needs to live in the Steam application directory.
- Every source workspace has a Vapor manifest and Cargo manifest at
  the same root; every Vapor project is represented by a Cargo package.
- Vapor application source roots are source super-repositories; they are not
  the same thing as the Steam installation/app root.
- Deleting or rebuilding `cargo metadata` output does not lose authored source;
  deleting either source manifest invalidates the workspace or project.
- Missing Vapor-home Cargo remains diagnosable and explicitly repairable, but
  Cargo-backed workflows cannot proceed or fall back to host Cargo.
- `vapor metadata` reports partial state in human-readable or JSON form.
- Commands validate only their own prerequisites and never repair them implicitly.
- User paths are canonicalized before source-boundary checks, including symlinks.
- Nested content markers update context; nested workspace markers are rejected.
- Vapor Shell can start closed with only an app root. Source work begins only
  after `source open SOURCE`, and opening a nested shell checkout selects the
  highest containing Vapor source root.

## Bootstrap and validate

Install or repair Vapor with the standalone Installer. Open the Shell from
Vapor home, then select a source workspace:

```text
vapor source open /path/to/source
fmt
validate
content validate
content install ghf-studios/loo-cast/loo-cast-packagepack
content verify
```

Vapor executables, managed toolchains, caches, sources, and mutable state stay
under Vapor home. A selected Steam product root is external content context and
is never a Vapor installation target.