//! Local client for requesting explicitly approved, temporary Platform SSH access.

use super::{
    PlatformCommand, PlatformSshCapability, PlatformSshCommand, PlatformSshCredentialCommand,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use vapor_core::VaporInstallation;

const DEFAULT_PLATFORM_ORIGIN: &str = "https://vapor.ghf-studios.site";
const REQUEST_SCOPE: &str = "platform:ssh-access:request";

#[derive(Serialize)]
struct CreateRequest<'a> {
    public_key: &'a str,
    purpose: &'a str,
    requested_capabilities: Vec<&'static str>,
    requested_duration_seconds: i64,
}

#[derive(Deserialize)]
struct CreatedRequest {
    id: String,
    fingerprint: String,
    status: String,
}

/// Execute a Platform SSH operation.
pub(crate) fn execute(command: PlatformCommand) -> Result<(), String> {
    let PlatformCommand::Ssh { command } = command;
    let installation = VaporInstallation::discover()
        .map_err(|error| format!("could not locate the Vapor installation: {error}"))?;
    let store = LocalStore::new(installation.user_data_root());

    match command {
        PlatformSshCommand::Credential { command } => match command {
            PlatformSshCredentialCommand::Set => store.set_credential(),
            PlatformSshCredentialCommand::Status => {
                if store.credential_path().is_file() {
                    println!("Platform SSH request credential: configured");
                } else {
                    println!("Platform SSH request credential: not configured");
                    println!(
                        "Create a credential with the `{REQUEST_SCOPE}` scope in Identity, then run `vapor platform ssh credential set`."
                    );
                }
                Ok(())
            }
            PlatformSshCredentialCommand::Clear => store.clear_credential(),
        },
        PlatformSshCommand::Request {
            purpose,
            duration_hours,
            capability,
        } => store.request(purpose, duration_hours, capability),
        PlatformSshCommand::RemoveKey => store.remove_key(),
    }
}

struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    fn new(user_data_root: PathBuf) -> Self {
        Self {
            root: user_data_root.join("platform/ssh-access"),
        }
    }

    fn credential_path(&self) -> PathBuf {
        self.root.join("request-credential")
    }

    fn key_path(&self) -> PathBuf {
        self.root.join("id_ed25519")
    }

    fn ensure_private_directory(&self) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("could not create {}: {error}", self.root.display()))?;
        set_private_directory_permissions(&self.root)?;
        Ok(())
    }

    fn set_credential(&self) -> Result<(), String> {
        let token = rpassword::prompt_password("One-time Platform request credential: ")
            .map_err(|error| format!("could not read the credential securely: {error}"))?;
        let token = token.trim();
        if !valid_request_credential(token) {
            return Err("that does not look like a Vapor request credential; it must start with `vap_req_` and contain 64 hexadecimal characters".to_owned());
        }
        self.ensure_private_directory()?;
        write_private_file(&self.credential_path(), token.as_bytes())?;
        println!(
            "Platform SSH request credential stored for this user. It was not displayed or added to shell history."
        );
        Ok(())
    }

    fn clear_credential(&self) -> Result<(), String> {
        match fs::remove_file(self.credential_path()) {
            Ok(()) => println!("Platform SSH request credential removed."),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                println!("No local Platform SSH request credential was stored.");
            }
            Err(error) => return Err(format!("could not remove the stored credential: {error}")),
        }
        Ok(())
    }

    fn remove_key(&self) -> Result<(), String> {
        let private = self.key_path();
        for path in [&private, &public_key_path(&private)] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("could not remove {}: {error}", path.display()));
                }
            }
        }
        println!(
            "Local Platform SSH key removed. Any already-approved server grant remains until its approved expiry or operator revocation."
        );
        Ok(())
    }

    fn request(
        &self,
        purpose: String,
        duration_hours: u8,
        capability: Vec<PlatformSshCapability>,
    ) -> Result<(), String> {
        let token = fs::read_to_string(self.credential_path()).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!("no Platform SSH request credential is configured; create one with the `{REQUEST_SCOPE}` scope in Identity, then run `vapor platform ssh credential set`")
            } else {
                format!("could not read the local Platform SSH request credential: {error}")
            }
        })?;
        let token = token.trim();
        if !valid_request_credential(token) {
            return Err("the local Platform SSH request credential is invalid; replace it with `vapor platform ssh credential set`".to_owned());
        }
        if purpose.trim().is_empty() {
            return Err("the SSH request purpose cannot be empty".to_owned());
        }
        if capability.is_empty() {
            return Err("select at least one SSH capability".to_owned());
        }
        if capability.contains(&PlatformSshCapability::HostAdmin) && capability.len() != 1 {
            return Err("`host-admin` must be requested by itself; the server requires a separate approval confirmation for it".to_owned());
        }

        self.ensure_private_directory()?;
        let private_key = self.ensure_key_pair()?;
        let public_key = fs::read_to_string(public_key_path(&private_key))
            .map_err(|error| format!("could not read the SSH public key: {error}"))?;
        let origin = platform_origin()?;
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("could not prepare a secure Platform connection: {error}"))?;
        let request = CreateRequest {
            public_key: public_key.trim(),
            purpose: purpose.trim(),
            requested_capabilities: capability
                .into_iter()
                .map(PlatformSshCapability::as_api_value)
                .collect(),
            requested_duration_seconds: i64::from(duration_hours) * 60 * 60,
        };
        let response = client
            .post(format!("{origin}/api/ssh-access/requests"))
            .bearer_auth(token)
            .json(&request)
            .send()
            .map_err(|error| {
                format!("could not submit the SSH request to Vapor Platform: {error}")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let detail = response
                .text()
                .unwrap_or_default()
                .trim()
                .chars()
                .take(300)
                .collect::<String>();
            let hint = if status == reqwest::StatusCode::UNAUTHORIZED {
                " The request credential may have expired or been revoked; replace it from Identity."
            } else {
                ""
            };
            return Err(format!(
                "Platform rejected the SSH request ({status}): {detail}.{hint}"
            ));
        }
        let created: CreatedRequest = response.json().map_err(|error| {
            format!("Platform accepted the request but returned an unreadable response: {error}")
        })?;

        println!("Temporary SSH request submitted.");
        println!("  Request: {}", created.id);
        println!("  Status: {}", created.status);
        println!("  Key fingerprint: {}", created.fingerprint);
        println!("  Local key: {}", private_key.display());
        println!("  Review and approve: {origin}/admin/operations#ssh-access");
        println!("  Requested duration: {duration_hours} hour(s)");
        println!(
            "  Requested capabilities: {}",
            request.requested_capabilities.join(", ")
        );
        Ok(())
    }

    fn ensure_key_pair(&self) -> Result<PathBuf, String> {
        let private = self.key_path();
        let public = public_key_path(&private);
        if private.is_file() && public.is_file() {
            validate_public_key(&public)?;
            return Ok(private);
        }
        if private.exists() && !public.exists() {
            let output = Command::new("ssh-keygen")
                .arg("-y")
                .arg("-f")
                .arg(&private)
                .output()
                .map_err(|error| {
                    format!("could not run OpenSSH `ssh-keygen` to restore the public key: {error}")
                })?;
            if !output.status.success() {
                return Err("the stored SSH private key could not be read by OpenSSH; remove it with `vapor platform ssh remove-key` and retry".to_owned());
            }
            write_private_file(&public, &output.stdout)?;
            validate_public_key(&public)?;
            return Ok(private);
        }
        if private.exists() || public.exists() {
            return Err("the local SSH key files are incomplete; remove them with `vapor platform ssh remove-key` and retry".to_owned());
        }

        let output = Command::new("ssh-keygen")
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "Vapor temporary Platform access",
                "-f",
            ])
            .arg(&private)
            .output()
            .map_err(|error| {
                format!(
                    "OpenSSH `ssh-keygen` is required to create the dedicated access key: {error}"
                )
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(format!(
                "could not create the dedicated SSH key{}",
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            ));
        }
        set_private_file_permissions(&private)?;
        validate_public_key(&public)?;
        Ok(private)
    }
}

fn platform_origin() -> Result<String, String> {
    let configured =
        std::env::var("VAPOR_PLATFORM_URL").unwrap_or_else(|_| DEFAULT_PLATFORM_ORIGIN.to_owned());
    let url = reqwest::Url::parse(configured.trim_end_matches('/'))
        .map_err(|_| "VAPOR_PLATFORM_URL must be a valid origin URL".to_owned())?;
    let local_host = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    let is_local_http = url.scheme() == "http" && local_host;
    if (url.scheme() != "https" && !is_local_http)
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "VAPOR_PLATFORM_URL must be a plain HTTPS origin (HTTP is allowed only for loopback development)".to_owned(),
        );
    }
    Ok(url.origin().ascii_serialization())
}

fn valid_request_credential(token: &str) -> bool {
    token.len() == 72
        && token
            .strip_prefix("vap_req_")
            .is_some_and(|suffix| suffix.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn public_key_path(private_key: &Path) -> PathBuf {
    PathBuf::from(format!("{}.pub", private_key.display()))
}

fn validate_public_key(path: &Path) -> Result<(), String> {
    let key = fs::read_to_string(path)
        .map_err(|error| format!("could not read SSH public key {}: {error}", path.display()))?;
    if !key.trim_start().starts_with("ssh-ed25519 ") {
        return Err(format!("{} is not an ssh-ed25519 key", path.display()));
    }
    Ok(())
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        format!(
            "could not create protected file {}: {error}",
            temporary.display()
        )
    })?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "could not securely write {}: {error}",
            path.display()
        ));
    }
    set_private_file_permissions(&temporary)?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not store {}: {error}", path.display())
    })
}

fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("could not protect {}: {error}", path.display()))?;
    }
    // On Windows the directory lives below the current user's LocalAppData,
    // whose inherited ACL is scoped to that user and the local system.
    #[cfg(windows)]
    let _ = path;
    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("could not protect {}: {error}", path.display()))?;
    }
    // The key and credential inherit the user-only LocalAppData ACL on Windows.
    #[cfg(windows)]
    let _ = path;
    Ok(())
}
