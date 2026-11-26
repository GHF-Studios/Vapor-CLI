# Vapor-CLI

Vapor-CLI is the terminal facade for Vapor. It owns the interactive shell,
command grammar, and terminal output. Reusable operations live in Vapor; SDK
authoring commands are available through `vapor sdk` in the same process.

The shipped terminal binary remains `vapor`.

## Temporary Platform SSH access

The client can submit a temporary SSH request for the current machine. Each
request begins in the CLI and is authenticated by a one-time browser
authorization in Vapor Identity. The browser shows the SSH key fingerprint,
purpose, requested capabilities, and requested duration. Identity issues a
single-use, three-minute request credential bound to that SSH key; the CLI
redeems it immediately and does not save it. The Platform operator must still
approve the actual SSH grant in the Platform dashboard.

Run:

   ```text
   vapor platform ssh request
   ```

The CLI generates a dedicated Ed25519 key under the current user's Vapor data
directory. It sends only the public key; the private key remains local. The
default request is eight hours with deployment and service-control capabilities.
Use `--duration-hours` and repeat `--capability` to adjust it. A Platform
operator may change the requested scope or duration, up to the server's
eight-hour policy; granting `host-admin` requires a separate confirmation.
The CLI prints the request ID, key fingerprint, key path, and dashboard link.
Remove the local key with `vapor platform ssh remove-key`.

Set `VAPOR_PLATFORM_URL` only for a trusted HTTPS deployment or a loopback
development server.
