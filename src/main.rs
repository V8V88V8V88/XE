mod ast;
mod codegen;
mod compiler;
mod error;
mod lexer;
mod parser;
mod semantic;

use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use compiler::{compile_path, CompilationFailure};

fn print_usage() {
    eprintln!("XE Programming Language Compiler");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  xe compile <file.xe>           Compile and print generated Rust code");
    eprintln!("  xe compile <file.xe> -o <out>  Compile and build a native executable");
    eprintln!(
        "  xe install [--to <dir>]        Install the current XE binary into a local bin directory"
    );
    eprintln!("  xe run <file.xe>               Compile and run the program");
    eprintln!("  xe update                      Check for updates and install the latest version");
    eprintln!("  xe help                        Show this help message");
    eprintln!("  xe --version, -v               Show the version of the compiler");
}

fn print_compile_error(error: &CompilationFailure) {
    if error.source.is_empty() {
        eprintln!("{}", error.error);
    } else {
        eprintln!("{}", error.error.render_with_source(&error.source));
    }
}

fn clean_temp_dir(path: &std::path::Path) {
    let _ = fs::remove_dir_all(path);
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn cargo_bin_dir() -> Option<PathBuf> {
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        return Some(PathBuf::from(cargo_home).join("bin"));
    }

    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home).join(".cargo").join("bin"));
    }

    #[cfg(windows)]
    if let Ok(profile) = env::var("USERPROFILE") {
        return Some(PathBuf::from(profile).join(".cargo").join("bin"));
    }

    None
}

fn command_available(program: &Path) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn update_xe() {
    println!("Checking for updates...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("V8V88V8V88")
        .repo_name("XE")
        .bin_name("xe")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()
        .and_then(|update| update.update());

    match status {
        Ok(status) => {
            if status.updated() {
                println!("Successfully updated to version {}!", status.version());
            } else {
                println!("XE is already up to date (version {}).", status.version());
            }
        }
        Err(e) => {
            eprintln!("Failed to update XE: {}", e);
            std::process::exit(1);
        }
    }
}

fn rustc_executable_name() -> &'static str {
    if cfg!(windows) {
        "rustc.exe"
    } else {
        "rustc"
    }
}

fn resolve_rustc() -> Option<PathBuf> {
    let path_lookup = PathBuf::from(rustc_executable_name());
    if command_available(&path_lookup) {
        return Some(path_lookup);
    }

    let cargo_rustc = cargo_bin_dir()?.join(rustc_executable_name());
    if command_available(&cargo_rustc) {
        return Some(cargo_rustc);
    }

    None
}

fn refresh_path_with_cargo_bin() {
    let Some(cargo_bin) = cargo_bin_dir() else {
        return;
    };

    let mut paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();

    if !paths.iter().any(|existing| existing == &cargo_bin) {
        paths.insert(0, cargo_bin);
        if let Ok(joined) = env::join_paths(paths) {
            env::set_var("PATH", joined);
        }
    }
}

#[cfg(unix)]
fn install_rust_toolchain() -> Result<(), String> {
    let shell_script =
        "if command -v curl >/dev/null 2>&1; then curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
         elif command -v wget >/dev/null 2>&1; then wget -qO- https://sh.rustup.rs | sh -s -- -y; \
         else exit 127; fi";

    let status = Command::new("sh")
        .arg("-c")
        .arg(shell_script)
        .status()
        .map_err(|e| format!("failed to start rustup installer: {}", e))?;

    if status.success() {
        Ok(())
    } else if status.code() == Some(127) {
        Err("could not find curl or wget to download Rust".to_string())
    } else {
        Err(format!("rustup installer exited with status {}", status))
    }
}

#[cfg(windows)]
fn install_rust_toolchain() -> Result<(), String> {
    let temp_installer = env::temp_dir().join(format!("rustup-init-{}.exe", std::process::id()));
    let installer_str = temp_installer.to_string_lossy().replace('\'', "''");
    let command = format!(
        "$ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile '{}'; \
         Start-Process -Wait -FilePath '{}' -ArgumentList '-y', '--default-host', 'x86_64-pc-windows-gnu', '--profile', 'minimal';",
        installer_str, installer_str
    );

    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command)
        .status()
        .map_err(|e| format!("failed to start PowerShell Rust installer: {}", e))?;

    let _ = fs::remove_file(&temp_installer);

    if status.success() {
        Ok(())
    } else {
        Err(format!("rustup installer exited with status {}", status))
    }
}

fn ensure_rustc_available() -> PathBuf {
    if let Some(rustc) = resolve_rustc() {
        return rustc;
    }

    eprintln!("Rust compiler not found. Installing Rust toolchain with rustup...");
    if let Err(message) = install_rust_toolchain() {
        eprintln!("Error: {}", message);
        eprintln!("Install Rust manually from https://rustup.rs/ and run the command again.");
        std::process::exit(1);
    }

    refresh_path_with_cargo_bin();

    if let Some(rustc) = resolve_rustc() {
        eprintln!("Rust toolchain installed successfully.");
        return rustc;
    }

    eprintln!("Error: Rust installation completed, but rustc is still not available.");
    eprintln!("Open a new shell or add Cargo's bin directory to PATH, then retry.");
    std::process::exit(1);
}

#[cfg(unix)]
fn default_install_dir() -> Result<PathBuf, String> {
    let home = env::var("HOME")
        .map_err(|_| "could not determine the home directory for installation".to_string())?;
    Ok(PathBuf::from(home).join(".local").join("bin"))
}

#[cfg(windows)]
fn default_install_dir() -> Result<PathBuf, String> {
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        return Ok(PathBuf::from(local_app_data)
            .join("Programs")
            .join("XE")
            .join("bin"));
    }

    if let Ok(profile) = env::var("USERPROFILE") {
        return Ok(PathBuf::from(profile)
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("XE")
            .join("bin"));
    }

    Err("could not determine the local application data directory for installation".to_string())
}

fn resolve_install_source() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("failed to determine current executable path: {}", e))?;

    let executable_name = if cfg!(windows) { "xe.exe" } else { "xe" };

    if current_exe
        .components()
        .any(|component| component.as_os_str() == "debug")
    {
        if let Some(debug_dir) = current_exe.parent() {
            if let Some(target_dir) = debug_dir.parent() {
                let release_candidate = target_dir.join("release").join(executable_name);
                if release_candidate.exists() {
                    return Ok(release_candidate);
                }
            }
        }
    }

    Ok(current_exe)
}

fn current_path_contains(dir: &Path) -> bool {
    env::var_os("PATH")
        .map(|value| env::split_paths(&value).any(|entry| entry == dir))
        .unwrap_or(false)
}

fn prepend_to_current_path(dir: &Path) {
    if current_path_contains(dir) {
        return;
    }

    let mut paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();
    paths.insert(0, dir.to_path_buf());

    if let Ok(joined) = env::join_paths(paths) {
        env::set_var("PATH", joined);
    }
}

#[cfg(unix)]
fn shell_profile_path() -> Option<PathBuf> {
    let home = PathBuf::from(env::var("HOME").ok()?);
    let shell = env::var("SHELL").unwrap_or_default();

    if shell.ends_with("zsh") {
        return Some(home.join(".zprofile"));
    }

    if shell.ends_with("bash") {
        let bash_profile = home.join(".bash_profile");
        if bash_profile.exists() {
            return Some(bash_profile);
        }
    }

    Some(home.join(".profile"))
}

#[cfg(unix)]
fn path_export_line(dir: &Path) -> String {
    if let Ok(home) = env::var("HOME") {
        let home_path = PathBuf::from(&home);
        if let Ok(relative) = dir.strip_prefix(&home_path) {
            if !relative.as_os_str().is_empty() {
                let relative = relative.to_string_lossy().replace('\\', "/");
                return format!("export PATH=\"$HOME/{}:$PATH\"", relative);
            }
        }
    }

    format!("export PATH={}:$PATH", shell_quote(&dir.to_string_lossy()))
}

#[cfg(unix)]
fn persist_path_entry(dir: &Path) -> Result<bool, String> {
    let profile = shell_profile_path()
        .ok_or_else(|| "could not determine which shell profile to update".to_string())?;
    let export_line = path_export_line(dir);
    let existing = fs::read_to_string(&profile).unwrap_or_default();

    if existing.contains(&export_line) {
        return Ok(false);
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str("# Added by XE installer\n");
    updated.push_str(&export_line);
    updated.push('\n');

    fs::write(&profile, updated)
        .map_err(|e| format!("failed to update '{}': {}", profile.display(), e))?;
    Ok(true)
}

#[cfg(windows)]
fn persist_path_entry(dir: &Path) -> Result<bool, String> {
    let dir_str = dir.to_string_lossy().replace('\'', "''");
    let command = format!(
        "$dir = '{}'; \
         $current = [Environment]::GetEnvironmentVariable('Path', 'User'); \
         $entries = @(); \
         if ($current) {{ $entries = $current.Split(';') | Where-Object {{ $_ }} }}; \
         if ($entries -contains $dir) {{ exit 0 }}; \
         $newPath = if ([string]::IsNullOrEmpty($current)) {{ $dir }} else {{ \"$dir;$current\" }}; \
         [Environment]::SetEnvironmentVariable('Path', $newPath, 'User'); \
         exit 10;",
        dir_str
    );

    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command)
        .status()
        .map_err(|e| format!("failed to update the user PATH: {}", e))?;

    match status.code() {
        Some(0) => Ok(false),
        Some(10) => Ok(true),
        _ => Err(format!(
            "failed to update the user PATH (status {})",
            status
        )),
    }
}

fn prompt_yes_no(message: &str) -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }

    eprint!("{} [y/N]: ", message);
    let _ = io::stderr().flush();

    let mut response = String::new();
    if io::stdin().read_line(&mut response).is_err() {
        return false;
    }

    matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

#[cfg(unix)]
fn relaunch_install_with_elevation(args: &[String]) -> Result<i32, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("failed to determine current executable path: {}", e))?;
    let status = Command::new("sudo")
        .arg(current_exe)
        .arg("install")
        .args(args)
        .status()
        .map_err(|e| format!("failed to start elevated installer with sudo: {}", e))?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(windows)]
fn relaunch_install_with_elevation(args: &[String]) -> Result<i32, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("failed to determine current executable path: {}", e))?;
    let exe = current_exe.to_string_lossy().replace('\'', "''");
    let argument_items = std::iter::once("install".to_string())
        .chain(args.iter().cloned())
        .map(|arg| format!("'{}'", arg.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");
    let command = format!(
        "$proc = Start-Process -Verb RunAs -Wait -PassThru -FilePath '{}' -ArgumentList @({}); exit $proc.ExitCode",
        exe, argument_items
    );

    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command)
        .status()
        .map_err(|e| format!("failed to start elevated installer with UAC: {}", e))?;
    Ok(status.code().unwrap_or(1))
}

fn exit_with_permission_guidance(args: &[String], install_dir: &Path, operation: &str) -> ! {
    let target = install_dir.display();
    let guidance = if cfg!(windows) {
        format!(
            "Permission denied while trying to {} '{}'. Relaunch with administrator access?",
            operation, target
        )
    } else {
        format!(
            "Permission denied while trying to {} '{}'. Retry with sudo?",
            operation, target
        )
    };

    if prompt_yes_no(&guidance) {
        match relaunch_install_with_elevation(args) {
            Ok(code) => std::process::exit(code),
            Err(message) => {
                eprintln!("Error: {}", message);
                std::process::exit(1);
            }
        }
    }

    if cfg!(windows) {
        eprintln!(
            "Error: permission denied while trying to {} '{}'. Run the command again from an Administrator shell or choose a user-writable directory.",
            operation, target
        );
    } else {
        eprintln!(
            "Error: permission denied while trying to {} '{}'. Run the command again with sudo or choose a user-writable directory.",
            operation, target
        );
    }
    std::process::exit(1);
}

fn install_binary(args: &[String]) {
    let install_dir = match args {
        [] => match default_install_dir() {
            Ok(dir) => dir,
            Err(message) => {
                eprintln!("Error: {}", message);
                std::process::exit(1);
            }
        },
        [flag, dir] if flag == "--to" => PathBuf::from(dir),
        _ => {
            eprintln!("Error: Invalid install arguments");
            eprintln!("Usage: xe install [--to <directory>]");
            std::process::exit(1);
        }
    };

    let source = match resolve_install_source() {
        Ok(path) => path,
        Err(message) => {
            eprintln!("Error: {}", message);
            std::process::exit(1);
        }
    };

    if let Err(e) = fs::create_dir_all(&install_dir) {
        if e.kind() == io::ErrorKind::PermissionDenied {
            exit_with_permission_guidance(args, &install_dir, "create");
        }
        eprintln!(
            "Error: Failed to create install directory '{}': {}",
            install_dir.display(),
            e
        );
        std::process::exit(1);
    }

    let target_name = if cfg!(windows) { "xe.exe" } else { "xe" };
    let target = install_dir.join(target_name);

    if let Err(e) = fs::copy(&source, &target) {
        if e.kind() == io::ErrorKind::PermissionDenied {
            exit_with_permission_guidance(args, &target, "write");
        }
        eprintln!(
            "Error: Failed to install '{}' to '{}': {}",
            source.display(),
            target.display(),
            e
        );
        std::process::exit(1);
    }

    #[cfg(unix)]
    if let Err(e) = fs::set_permissions(&target, fs::Permissions::from_mode(0o755)) {
        if e.kind() == io::ErrorKind::PermissionDenied {
            exit_with_permission_guidance(args, &target, "set permissions on");
        }
        eprintln!(
            "Error: Installed binary at '{}' but could not update permissions: {}",
            target.display(),
            e
        );
        std::process::exit(1);
    }

    prepend_to_current_path(&install_dir);

    eprintln!("Installed XE to {}", target.display());
    match persist_path_entry(&install_dir) {
        Ok(true) => {
            if cfg!(windows) {
                eprintln!(
                    "Added '{}' to your user PATH. Open a new shell to use 'xe' directly.",
                    install_dir.display()
                );
            } else {
                eprintln!(
                    "Added '{}' to your shell PATH configuration. Open a new shell to use 'xe' directly.",
                    install_dir.display()
                );
            }
        }
        Ok(false) => {
            if current_path_contains(&install_dir) {
                eprintln!("'{}' is already available in PATH.", install_dir.display());
            } else {
                eprintln!(
                    "'{}' is already configured for future shells. Open a new shell to use 'xe' directly.",
                    install_dir.display()
                );
            }
        }
        Err(message) => {
            eprintln!("Warning: {}", message);
            eprintln!(
                "Add '{}' to your PATH manually if you want to run 'xe' directly in new shells.",
                install_dir.display()
            );
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-v" => {
            println!("xe version {}", env!("CARGO_PKG_VERSION"));
        }
        "update" => {
            update_xe();
        }
        "install" => {
            install_binary(&args[2..]);
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("Error: No input file specified");
                eprintln!("Usage: xe compile <file.xe> [-o <output>]");
                std::process::exit(1);
            }

            let input_file = Path::new(&args[2]);

            match compile_path(input_file) {
                Ok(rust_code) => {
                    if args.len() == 3 {
                        print!("{}", rust_code);
                    } else if args.len() == 5 && args[3] == "-o" {
                        let output_file = &args[4];
                        let rustc = ensure_rustc_available();

                        // Create a temporary .rs file
                        let temp_rs = format!("{}.rs", output_file);
                        if let Err(e) = fs::write(&temp_rs, &rust_code) {
                            eprintln!("Error writing intermediate file: {}", e);
                            std::process::exit(1);
                        }

                        // Call rustc to create the final binary
                        let rustc_output = Command::new(&rustc)
                            .arg(&temp_rs)
                            .arg("-o")
                            .arg(output_file)
                            .arg("-C")
                            .arg("opt-level=3")
                            .arg("--edition")
                            .arg("2021")
                            .stderr(Stdio::piped())
                            .output();

                        // Clean up the temporary .rs file
                        let _ = fs::remove_file(&temp_rs);

                        match rustc_output {
                            Ok(output) if output.status.success() => {
                                eprintln!("Successfully compiled to binary: {}", output_file);
                            }
                            Ok(output) => {
                                eprintln!("Rust compilation failed:");
                                io::stderr().write_all(&output.stderr).unwrap();
                                std::process::exit(1);
                            }
                            Err(e) => {
                                eprintln!("Error: Failed to run rustc: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Error: Invalid compile arguments");
                        eprintln!("Usage: xe compile <file.xe> [-o <output>]");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    print_compile_error(&e);
                    std::process::exit(1);
                }
            }
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Error: No input file specified");
                eprintln!("Usage: xe run <file.xe>");
                std::process::exit(1);
            }

            let input_file = Path::new(&args[2]);

            let rust_code = match compile_path(input_file) {
                Ok(code) => code,
                Err(e) => {
                    print_compile_error(&e);
                    std::process::exit(1);
                }
            };

            // Create temp directory for compilation
            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let temp_dir = std::env::temp_dir().join(format!("xe_run_{}", unique_id));
            let _ = fs::create_dir_all(&temp_dir);

            let rust_file = temp_dir.join("main.rs");
            let exe_file = if cfg!(windows) {
                temp_dir.join("main.exe")
            } else {
                temp_dir.join("main")
            };
            let rustc = ensure_rustc_available();

            // Write Rust code
            if let Err(e) = fs::write(&rust_file, &rust_code) {
                eprintln!("Error writing temp file: {}", e);
                clean_temp_dir(&temp_dir);
                std::process::exit(1);
            }

            // Compile with rustc
            let compile_result = Command::new(&rustc)
                .arg(&rust_file)
                .arg("-o")
                .arg(&exe_file)
                .arg("-C")
                .arg("opt-level=3")
                .arg("--edition")
                .arg("2021")
                .stderr(Stdio::piped())
                .output();

            match compile_result {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("Rust compilation failed:");
                        io::stderr().write_all(&output.stderr).unwrap();
                        clean_temp_dir(&temp_dir);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run rustc: {}", e);
                    clean_temp_dir(&temp_dir);
                    std::process::exit(1);
                }
            }

            // Run the executable
            let run_result = Command::new(&exe_file)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();

            let exit_code = match run_result {
                Ok(status) => status.code().unwrap_or(1),
                Err(e) => {
                    eprintln!("Failed to run program: {}", e);
                    1
                }
            };

            clean_temp_dir(&temp_dir);
            std::process::exit(exit_code);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}
