use std::{
    ffi::OsString,
    fs,
    path::Path,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let Some(compiler) = std::env::var_os("VAPOR_CC_REAL_COMPILER") else {
        eprintln!("Vapor compiler wrapper is missing VAPOR_CC_REAL_COMPILER");
        return ExitCode::FAILURE;
    };

    let args = std::env::args_os()
        .skip(1)
        .map(strip_verbatim_prefix)
        .collect::<Vec<_>>();
    for argument in &args {
        if let Err(error) = normalize_response_file(argument) {
            eprintln!("Vapor compiler wrapper could not normalize linker arguments: {error}");
            return ExitCode::FAILURE;
        }
    }

    let mut command = Command::new(compiler);
    command.args(args);

    match command.status() {
        Ok(status) => status
            .code()
            .map(|code| ExitCode::from(code.clamp(0, 255) as u8))
            .unwrap_or(ExitCode::FAILURE),
        Err(error) => {
            eprintln!("Vapor could not start its managed C compiler: {error}");
            ExitCode::FAILURE
        }
    }
}

fn normalize_response_file(argument: &OsString) -> std::io::Result<()> {
    let Some(response_path) = argument
        .to_str()
        .and_then(|argument| argument.strip_prefix('@'))
    else {
        return Ok(());
    };

    let response_path = Path::new(response_path);
    let response = fs::read_to_string(response_path)?;
    let normalized = response
        .replace(r"\\\\?\\", "")
        .replace(r"\\?\", "");
    if normalized != response {
        fs::write(response_path, normalized)?;
    }
    Ok(())
}

#[cfg(windows)]
fn strip_verbatim_prefix(argument: std::ffi::OsString) -> std::ffi::OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    const PREFIX: [u16; 4] = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    let wide = argument.encode_wide().collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(wide.len());
    let mut index = 0;

    while index < wide.len() {
        if wide[index..].starts_with(&PREFIX) {
            index += PREFIX.len();
        } else {
            normalized.push(wide[index]);
            index += 1;
        }
    }

    std::ffi::OsString::from_wide(&normalized)
}

#[cfg(not(windows))]
fn strip_verbatim_prefix(argument: std::ffi::OsString) -> std::ffi::OsString {
    argument
}
