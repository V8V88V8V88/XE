mod ast;
mod codegen;
mod error;
mod lexer;
mod parser;
mod semantic;

use std::env;
use std::fs;
use std::io::{self, Write};
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
                    // Check for -o flag
                    if args.len() >= 5 && args[3] == "-o" {
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
                        print!("{}", rust_code);
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
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run rustc: {}", e);
                    eprintln!("Make sure Rust is installed and rustc is in your PATH");
                    std::process::exit(1);
                }
            }

            // Run the executable
            let run_result = Command::new(&exe_file)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();

            match run_result {
                Ok(status) => {
                    std::process::exit(status.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("Failed to run program: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}
