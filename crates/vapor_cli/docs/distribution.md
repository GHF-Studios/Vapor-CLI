# Steam app/depot content staging

`App-Source.vapor.toml` defines the Steam app payload with an explicit
include-list. This repository is the release source; product bytes come from
the installed content tree supplied as `VAPOR_PRODUCT_ROOT`. `App.vapor.toml`
inside the common depot declares the composed content mount.

The Loo-Cast composition has three content items: Spacetime Engine, Loo-Cast
game, and Loo-Cast Packagepack. The common depot carries their shared metadata
and files. The Linux and Windows depots carry only ABI-specific files for their
respective target triples. The manifest excludes engine/game `lib/` folders and
Packagepack `bin/` from common, then stages those target-specific subdirectories
into the matching platform depot.

Vapor runtime files are a separate standalone distribution. The Steam app must
not contain Vapor executables, launcher/entrypoint files, source checkouts,
docs, developer tools, toolchain caches, logs, receipts, or user state. Vapor
home owns those files and all mutable state. Steam's role is to install and
update the content composition; Vapor mounts it and runs it.

## Staging behavior

`root package --release-targets` rebuilds the allowlisted staging tree under
`output/root/content` in Vapor home. `root publish --dry-run --release-targets`
repeats the stage, validates the required manifest paths, fingerprints the
payload, and writes SteamPipe preview files without uploading. A real publish
re-stages and verifies the fingerprint before upload. Neither operation copies
from the full Vapor home or adds undeclared files.

The `root.runtime.targets` list describes ABI targets available for this
product. `--target TARGET` selects a deliberate subset for a preview; the real
release should include the complete declared target matrix. Target-specific
includes must name the same target as the depot's operating-system kind.

Vapor keeps source checkouts, compiler state, tools, extracted-source cache,
installation records, settings, logs, and generated output in Vapor home. The
product source tree is an explicit input, not the owner of Vapor state.

SteamPipe templates remain source-controlled under
`resources/steam/steampipe-templates/`. These templates generate the upload
instructions and are not automatically included in a depot.