mod ast;
mod codegen;
mod error;
mod lexer;
mod parser;
mod semantic;

use std::env;
use std::fs;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use codegen::CodeGenerator;
use error::XeError;
use lexer::Lexer;
use parser::Parser;
use semantic::SemanticAnalyzer;

fn compile(source: &str) -> Result<String, XeError> {
    // Lexing
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)?;

    // Code generation
    let mut codegen = CodeGenerator::new();
    let rust_code = codegen.generate(&program);

    Ok(rust_code)
}

fn print_usage() {
    eprintln!("XE Programming Language Compiler");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  xe compile <file.xe>           Compile and print generated Rust code");
    eprintln!("  xe compile <file.xe> -o <out>  Compile and build a native executable");
    eprintln!("  xe install [--to <dir>]        Install the current XE binary into a local bin directory");
    eprintln!("  xe run <file.xe>               Compile and run the program");
    eprintln!("  xe help                        Show this help message");
}

fn read_source(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn print_compile_error(source: &str, error: &XeError) {
    eprintln!("{}", error.render_with_source(source));
}

fn clean_temp_dir(path: &std::path::Path) {
    let _ = fs::remove_dir_all(path);
}

fn default_install_dir() -> Result<PathBuf, String> {
    if let Ok(home) = env::var("HOME") {
        return Ok(PathBuf::from(home).join(".local").join("bin"));
    }

    #[cfg(windows)]
    if let Ok(profile) = env::var("USERPROFILE") {
        return Ok(PathBuf::from(profile).join(".local").join("bin"));
    }

    Err("could not determine the home directory for installation".to_string())
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
        eprintln!(
            "Error: Installed binary at '{}' but could not update permissions: {}",
            target.display(),
            e
        );
        std::process::exit(1);
    }

    eprintln!("Installed XE to {}", target.display());
    eprintln!(
        "Add '{}' to your PATH if it is not already available in new shells.",
        install_dir.display()
    );
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
        "install" => {
            install_binary(&args[2..]);
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("Error: No input file specified");
                eprintln!("Usage: xe compile <file.xe> [-o <output>]");
                std::process::exit(1);
            }

            let input_file = &args[2];
            let source = read_source(input_file);

            match compile(&source) {
                Ok(rust_code) => {
                    if args.len() == 3 {
                        print!("{}", rust_code);
                    } else if args.len() == 5 && args[3] == "-o" {
                        let output_file = &args[4];

                        // Create a temporary .rs file
                        let temp_rs = format!("{}.rs", output_file);
                        if let Err(e) = fs::write(&temp_rs, &rust_code) {
                            eprintln!("Error writing intermediate file: {}", e);
                            std::process::exit(1);
                        }

                        // Call rustc to create the final binary
                        let rustc_output = Command::new("rustc")
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
                    print_compile_error(&source, &e);
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

            let input_file = &args[2];
            let source = read_source(input_file);

            let rust_code = match compile(&source) {
                Ok(code) => code,
                Err(e) => {
                    print_compile_error(&source, &e);
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

            // Write Rust code
            if let Err(e) = fs::write(&rust_file, &rust_code) {
                eprintln!("Error writing temp file: {}", e);
                clean_temp_dir(&temp_dir);
                std::process::exit(1);
            }

            // Compile with rustc
            let compile_result = Command::new("rustc")
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
                    eprintln!("Make sure Rust is installed and rustc is in your PATH");
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
