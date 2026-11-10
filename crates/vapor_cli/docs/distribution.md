# Steam app/depot content staging

`App-Source.vapor.toml` defines the Steam app's product inputs with an explicit
include-list. Loo Cast's current depot contract is a common
`Packagepack.vapor.toml` plus one primary Packagepack executable and only
declared required runtime libraries in the matching target depot.
`App.vapor.toml` is not part of this product contract.

The Loo-Cast workspace still authors three content items: Spacetime Engine,
Loo-Cast Game, and Loo-Cast Packagepack. The Packagepack Cargo binary directly
links the Engine and Game crates, so its target build is the composed runnable
product. Depot staging contains the Packagepack manifest, that target binary,
and any declared required runtime libraries. It rejects component
directories, loose Engine/Game libraries, Vapor files, and undeclared files.

Vapor runtime files are a separate standalone distribution. The Steam app must
not contain the Vapor runtime, source checkouts, docs, developer tools,
toolchain caches, logs, receipts, or user state. Steam launches the Packagepack
executable directly. Vapor home owns Vapor files and mutable state.

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
