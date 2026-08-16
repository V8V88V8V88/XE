use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_unique_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[allow(dead_code)]
fn compile_xe(source: &str) -> Result<String, String> {
    let id = get_unique_id();
    let temp_file = std::env::temp_dir().join(format!("test_input_{}.xe", id));
    fs::write(&temp_file, source).map_err(|e| e.to_string())?;

    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("compile")
        .arg(&temp_file)
        .output()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_file(&temp_file);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn run_xe(source: &str) -> Result<String, String> {
    run_xe_with_input(source, "")
}

fn run_xe_with_input(source: &str, input: &str) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let xe_file = temp_dir.join("input.xe");
    fs::write(&xe_file, source).map_err(|e| e.to_string())?;

    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("run")
        .arg(&xe_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    stdin.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
    drop(stdin);

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&temp_dir);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn compile_and_run_binary(source: &str) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_compile_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let xe_file = temp_dir.join("input.xe");
    fs::write(&xe_file, source).map_err(|e| e.to_string())?;

    let binary_path = if cfg!(windows) {
        temp_dir.join("program.exe")
    } else {
        temp_dir.join("program")
    };

    let compile_output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("compile")
        .arg(&xe_file)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !compile_output.status.success() {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(String::from_utf8_lossy(&compile_output.stderr).to_string());
    }

    let run_output = Command::new(&binary_path)
        .output()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&temp_dir);

    if run_output.status.success() {
        Ok(String::from_utf8_lossy(&run_output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&run_output.stderr).to_string())
    }
}

fn run_cli(args: &[&std::ffi::OsStr]) -> Result<String, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn write_project_files(root: &std::path::Path, files: &[(&str, &str)]) -> Result<(), String> {
    for (relative_path, source) in files {
        let full_path = root.join(relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(full_path, source).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn run_xe_project(entry_file: &str, files: &[(&str, &str)]) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_project_run_{}", id));
    let _ = fs::create_dir_all(&temp_dir);
    write_project_files(&temp_dir, files)?;

    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("run")
        .arg(temp_dir.join(entry_file))
        .output()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&temp_dir);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn compile_xe_project(entry_file: &str, files: &[(&str, &str)]) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_project_compile_{}", id));
    let _ = fs::create_dir_all(&temp_dir);
    write_project_files(&temp_dir, files)?;

    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("compile")
        .arg(temp_dir.join(entry_file))
        .output()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&temp_dir);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn compile_and_run_project_binary(
    entry_file: &str,
    files: &[(&str, &str)],
) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_project_binary_{}", id));
    let _ = fs::create_dir_all(&temp_dir);
    write_project_files(&temp_dir, files)?;

    let binary_path = if cfg!(windows) {
        temp_dir.join("program.exe")
    } else {
        temp_dir.join("program")
    };

    let compile_output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("compile")
        .arg(temp_dir.join(entry_file))
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !compile_output.status.success() {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(String::from_utf8_lossy(&compile_output.stderr).to_string());
    }

    let run_output = Command::new(&binary_path)
        .output()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_dir_all(&temp_dir);

    if run_output.status.success() {
        Ok(String::from_utf8_lossy(&run_output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&run_output.stderr).to_string())
    }
}

#[test]
fn test_hello_world() {
    let output = run_xe(r#"print("Hello, World!")"#).unwrap();
    assert_eq!(output.trim(), "Hello, World!");
}

#[test]
fn test_arithmetic() {
    let output = run_xe("print(10 + 20)").unwrap();
    assert_eq!(output.trim(), "30");

    let output = run_xe("print(50 - 8)").unwrap();
    assert_eq!(output.trim(), "42");

    let output = run_xe("print(6 * 7)").unwrap();
    assert_eq!(output.trim(), "42");

    let output = run_xe("print(100 / 4)").unwrap();
    assert_eq!(output.trim(), "25");

    let output = run_xe("print(-5)").unwrap();
    assert_eq!(output.trim(), "-5");
}

#[test]
fn test_variables() {
    let output = run_xe(
        r#"
x = 5
y = 10
print(x + y)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "15");
}

#[test]
fn test_string_concatenation() {
    let output = run_xe(r#"print("Hello" + " " + "World")"#).unwrap();
    assert_eq!(output.trim(), "Hello World");
}

#[test]
fn test_if_statement() {
    let output = run_xe(
        r#"
x = 10
if x > 5:
    print("big")
else:
    print("small")
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "big");

    let output = run_xe(
        r#"
x = 3
if x > 5:
    print("big")
else:
    print("small")
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "small");
}

#[test]
fn test_repeat_loop() {
    let output = run_xe(
        r#"
repeat 3 times:
    print("hi")
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "hi\nhi\nhi");
}

#[test]
fn test_repeat_loop_can_reassign_outer_variable() {
    let output = run_xe(
        r#"
count = 0
repeat 5 times:
    count = count + 1
print(count)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "5");
}

#[test]
fn test_function_definition() {
    let output = run_xe(
        r#"
fun double(n):
    return n * 2

print(double(21))
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "42");
}

#[test]
fn test_assignment_in_if_updates_outer_scope() {
    let output = run_xe(
        r#"
x = 1
if true:
    x = 2
print(x)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_elif_chain() {
    let output = run_xe(
        r#"
score = 82

if score >= 90:
    print("A")
elif score >= 80:
    print("B")
elif score >= 70:
    print("C")
else:
    print("D")
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "B");
}

#[test]
fn test_while_loop() {
    let output = run_xe(
        r#"
count = 0
total = 0

while count < 5:
    total = total + count
    count = count + 1

print(total)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "10");
}

#[test]
fn test_for_loop_over_list() {
    let output = run_xe(
        r#"
total = 0

for item in [1, 2, 3, 4]:
    total = total + item

print(total)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "10");
}

#[test]
fn test_for_loop_over_text() {
    let output = run_xe(
        r#"
result = ""

for ch in "XE":
    result = result + ch

print(result)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "XE");
}

#[test]
fn test_break_in_while_loop() {
    let output = run_xe(
        r#"
count = 0

while true:
    count = count + 1
    if count == 3:
        break

print(count)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "3");
}

#[test]
fn test_continue_in_while_loop() {
    let output = run_xe(
        r#"
count = 0
total = 0

while count < 5:
    count = count + 1
    if count == 3:
        continue
    total = total + count

print(total)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "12");
}

#[test]
fn test_break_and_continue_in_for_loop() {
    let output = run_xe(
        r#"
result = ""

for ch in "ABCDE":
    if ch == "B":
        continue
    if ch == "D":
        break
    result = result + ch

print(result)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "AC");
}

#[test]
fn test_boolean_operations() {
    let output = run_xe("print(true and false)").unwrap();
    assert_eq!(output.trim(), "false");

    let output = run_xe("print(true or false)").unwrap();
    assert_eq!(output.trim(), "true");

    let output = run_xe("print(not false)").unwrap();
    assert_eq!(output.trim(), "true");
}

#[test]
fn test_list_operations() {
    let output = run_xe(
        r#"
items = [1, 2, 3]
print(length(items))
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "3");
}

#[test]
fn test_list_concatenation() {
    let output = run_xe(
        r#"
items = [1, 2]
items = items + [3, 4]
print(length(items))
for i in items:
    print(i)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "4\n1\n2\n3\n4");
}

#[test]
fn test_list_concatenation_mixed() {
    let output = run_xe(
        r#"
tasks = []
tasks = tasks + ["buy milk"]
tasks = tasks + ["clean room"]
print(length(tasks))
for t in tasks:
    print(t)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "2\nbuy milk\nclean room");
}

#[test]
fn test_type_function() {
    let output = run_xe(r#"print(type(42))"#).unwrap();
    assert_eq!(output.trim(), "number");

    let output = run_xe(r#"print(type("hello"))"#).unwrap();
    assert_eq!(output.trim(), "text");

    let output = run_xe(r#"print(type(true))"#).unwrap();
    assert_eq!(output.trim(), "boolean");
}

#[test]
fn test_comparison_operators() {
    let output = run_xe("print(5 == 5)").unwrap();
    assert_eq!(output.trim(), "true");

    let output = run_xe("print(5 != 3)").unwrap();
    assert_eq!(output.trim(), "true");

    let output = run_xe("print(10 >= 10)").unwrap();
    assert_eq!(output.trim(), "true");

    let output = run_xe("print(5 <= 10)").unwrap();
    assert_eq!(output.trim(), "true");
}

#[test]
fn test_undefined_variable_error() {
    let result = run_xe("print(undefined_var)");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined variable"));
}

#[test]
fn test_undefined_function_error() {
    let result = run_xe("unknown_func()");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined function"));
}

#[test]
fn test_loop_variable_scope_is_local() {
    let result = run_xe(
        r#"
for item in [1, 2]:
    print(item)

print(item)
"#,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined variable"));
}

#[test]
fn test_if_block_variable_scope_is_local() {
    let result = run_xe(
        r#"
if true:
    inner = 42

print(inner)
"#,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined variable"));
}

#[test]
fn test_break_outside_loop_error() {
    let result = run_xe("break");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("break can only be used inside a loop"));
}

#[test]
fn test_continue_outside_loop_error() {
    let result = run_xe("continue");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("continue can only be used inside a loop"));
}

#[test]
fn test_compile_o_produces_runnable_binary() {
    let output = compile_and_run_binary(
        r#"
value = 40
value = value + 2
print(value)
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "42");
}

#[test]
fn test_compile_without_o_prints_rust_code() {
    let rust_code = compile_xe(r#"print("Hello")"#).unwrap();
    assert!(rust_code.contains("fn main()"));
    assert!(rust_code.contains("xe_builtin_print"));
}

#[test]
fn test_runtime_error_for_invalid_number_conversion() {
    let result = run_xe(r#"print(convert("abc", "number"))"#);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Runtime error: cannot convert text 'abc' to number"));
}

#[test]
fn test_runtime_error_for_invalid_length_argument() {
    let result = run_xe("print(length(true))");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Runtime error: length() expected text or list, got boolean"));
}

#[test]
fn test_runtime_error_for_division_by_zero() {
    let result = run_xe("print(10 / 0)");
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_for_invalid_repeat_count() {
    let result = run_xe(
        r#"
repeat 2.5 times:
    print("hi")
"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_for_out_of_bounds_index() {
    let result = run_xe(
        r#"
items = [1, 2]
print(items[5])
"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_for_negative_index() {
    let result = run_xe(
        r#"
items = [1, 2, 3]
print(items[-1])
"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_for_invalid_for_iteration() {
    let result = run_xe(
        r#"
for item in 42:
    print(item)
"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_runtime_error_for_invalid_arithmetic_types() {
    let result = run_xe(r#"print([1, 2] - 1)"#);
    assert!(result.is_err());
}

#[test]
fn test_compile_error_shows_source_snippet_and_caret() {
    let result = run_xe(
        r#"
print(missing_name)
"#,
    );
    let error = result.unwrap_err();
    assert!(error.contains("undefined variable 'missing_name'"));
    assert!(error.contains("print(missing_name)"));
    assert!(error.contains("^"));
}

#[test]
fn test_return_outside_function_error() {
    let result = run_xe("return 1");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("return can only be used inside a function"));
}

#[test]
fn test_duplicate_function_definition_error() {
    let result = run_xe(
        r#"
fun answer():
    return 1

fun answer():
    return 2
"#,
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("function 'answer' is already defined"));
}

#[test]
fn test_function_scope_captures_outer_variables() {
    let output = run_xe(
        r#"
x = 1

fun show():
    print(x)

show()
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "1");
}

#[test]
fn test_compile_rejects_unknown_extra_arguments() {
    let id = get_unique_id();
    let temp_file = std::env::temp_dir().join(format!("test_input_{}.xe", id));
    fs::write(&temp_file, "print(1)\n").unwrap();

    let result = run_cli(&[
        std::ffi::OsStr::new("compile"),
        temp_file.as_os_str(),
        std::ffi::OsStr::new("unexpected"),
    ]);

    let _ = fs::remove_file(&temp_file);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid compile arguments"));
}

#[test]
fn test_install_command_copies_executable() {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_install_test_{}", id));
    let install_dir = temp_dir.join("bin");

    let install_output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("install")
        .arg("--to")
        .arg(&install_dir)
        .output()
        .expect("install command should run");

    assert!(install_output.status.success());
    let install_stderr = String::from_utf8_lossy(&install_output.stderr);
    assert!(install_stderr.contains("Installed XE to"));

    let installed_binary = if cfg!(windows) {
        install_dir.join("xe.exe")
    } else {
        install_dir.join("xe")
    };

    assert!(installed_binary.exists());

    let help_output = Command::new(&installed_binary)
        .arg("help")
        .output()
        .expect("installed xe should run");

    assert!(help_output.status.success());
    let stderr = String::from_utf8_lossy(&help_output.stderr);
    assert!(stderr.contains("XE Programming Language Compiler"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_from_import_across_files_runs_in_run_mode() {
    let output = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from math_utils import double
print(double(21))
"#,
            ),
            (
                "math_utils.xe",
                r#"
fun double(n):
    return n * 2
"#,
            ),
        ],
    )
    .unwrap();

    assert_eq!(output.trim(), "42");
}

#[test]
fn test_import_all_brings_exported_functions_into_scope() {
    let output = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
import helpers
print(square(7))
"#,
            ),
            (
                "helpers.xe",
                r#"
fun square(n):
    return n * n
"#,
            ),
        ],
    )
    .unwrap();

    assert_eq!(output.trim(), "49");
}

#[test]
fn test_imported_module_functions_can_call_their_own_imports() {
    let output = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from math_ops import quadruple
print(quadruple(5))
"#,
            ),
            (
                "math_ops.xe",
                r#"
from math_utils import double

fun quadruple(n):
    return double(double(n))
"#,
            ),
            (
                "math_utils.xe",
                r#"
fun double(n):
    return n * 2
"#,
            ),
        ],
    )
    .unwrap();

    assert_eq!(output.trim(), "20");
}

#[test]
fn test_module_initialization_runs_once_in_dependency_order() {
    let output = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from a import run_a
from b import run_b

run_a()
run_b()
"#,
            ),
            (
                "a.xe",
                r#"
import shared

fun run_a():
    print("A")
"#,
            ),
            (
                "b.xe",
                r#"
import shared

fun run_b():
    print("B")
"#,
            ),
            (
                "shared.xe",
                r#"
print("shared")
"#,
            ),
        ],
    )
    .unwrap();

    assert_eq!(output.trim(), "shared\nA\nB");
}

#[test]
fn test_compile_mode_builds_runnable_binary_with_imports() {
    let output = compile_and_run_project_binary(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from strings import shout
print(shout("xe"))
"#,
            ),
            (
                "strings.xe",
                r#"
fun shout(value):
    return value + "!"
"#,
            ),
        ],
    )
    .unwrap();

    assert_eq!(output.trim(), "xe!");
}

#[test]
fn test_compile_without_o_prints_linked_rust_for_imports() {
    let rust_code = compile_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from math_utils import double
print(double(10))
"#,
            ),
            (
                "math_utils.xe",
                r#"
fun double(n):
    return n * 2
"#,
            ),
        ],
    )
    .unwrap();

    assert!(rust_code.contains("fn main()"));
    assert!(rust_code.contains("xe_m"));
}

#[test]
fn test_missing_module_reports_import_error() {
    let result = run_xe_project(
        "main.xe",
        &[(
            "main.xe",
            r#"
import missing
"#,
        )],
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("module 'missing' could not be found"));
}

#[test]
fn test_missing_export_reports_module_symbol_error() {
    let result = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from math_utils import triple
print(triple(3))
"#,
            ),
            (
                "math_utils.xe",
                r#"
fun double(n):
    return n * 2
"#,
            ),
        ],
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not export 'triple'"));
}

#[test]
fn test_imports_must_come_before_top_level_executable_statements() {
    let result = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
print("before")
import helpers
"#,
            ),
            (
                "helpers.xe",
                r#"
fun helper():
    return 1
"#,
            ),
        ],
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("import statements must appear before executable top-level statements"));
}

#[test]
fn test_circular_imports_are_rejected() {
    let result = run_xe_project(
        "main.xe",
        &[
            (
                "main.xe",
                r#"
from a import run
run()
"#,
            ),
            (
                "a.xe",
                r#"
import b

fun run():
    return 0
"#,
            ),
            (
                "b.xe",
                r#"
import a
"#,
            ),
        ],
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("circular import detected"));
}

#[test]
fn test_palindrome_checker() {
    let source = fs::read_to_string("examples/palindrome.xe").unwrap();
    let output = run_xe_with_input(&source, "racecar\n").unwrap();
    assert!(output.contains("Yes, 'racecar' is a palindrome!"));
    
    let output = run_xe_with_input(&source, "hello\n").unwrap();
    assert!(output.contains("No, 'hello' is not a palindrome."));
}

#[test]
fn test_prime_numbers() {
    let source = fs::read_to_string("examples/prime_numbers.xe").unwrap();
    let output = run_xe_with_input(&source, "10\n").unwrap();
    assert!(output.contains("2\n3\n5\n7"));
}

#[test]
fn test_multiplication_table() {
    let source = fs::read_to_string("examples/multiplication_table.xe").unwrap();
    let output = run_xe_with_input(&source, "3\n").unwrap();
    assert!(output.contains("1 2 3 \n2 4 6 \n3 6 9 "));
}

#[test]
fn test_grade_system() {
    let source = fs::read_to_string("examples/grades.xe").unwrap();
    let output = run_xe_with_input(&source, "Alice\n2\n80\n90\n").unwrap();
    assert!(output.contains("Student: Alice"));
    assert!(output.contains("Average: 85"));
    assert!(output.contains("Status: PASSED"));
}

#[test]
fn test_todo_list() {
    let source = fs::read_to_string("examples/todo.xe").unwrap();
    let output = run_xe_with_input(&source, "1\nbuy milk\n1\nclean room\n3\n").unwrap();
    assert!(output.contains("- buy milk"));
    assert!(output.contains("- clean room"));
    assert!(output.contains("You have 2 task(s)"));
}

#[test]
fn test_adventure_game() {
    let source = fs::read_to_string("examples/adventure.xe").unwrap();
    let output = run_xe_with_input(&source, "2\n1\n").unwrap();
    assert!(output.contains("You find a shiny gold coin"));
    assert!(output.contains("YOU WIN!"));
}

#[test]
fn test_unit_converter() {
    let source = fs::read_to_string("examples/converter.xe").unwrap();
    let output = run_xe_with_input(&source, "1\n100\n3\n").unwrap();
    assert!(output.contains("100 C is 212 F"));
}

#[test]
fn test_list_equality_and_inequality() {
    let output = run_xe(
        r#"
a = [1, 2]
b = [1, 2]
c = [3, 4]
empty1 = []
empty2 = []

if a == b:
    print("a==b: true")
else:
    print("a==b: false")

if a == c:
    print("a==c: true")
else:
    print("a==c: false")

if a != c:
    print("a!=c: true")
else:
    print("a!=c: false")

if empty1 == empty2:
    print("empty==empty: true")
else:
    print("empty==empty: false")
"#,
    )
    .unwrap();

    assert!(output.contains("a==b: true"));
    assert!(output.contains("a==c: false"));
    assert!(output.contains("a!=c: true"));
    assert!(output.contains("empty==empty: true"));
}

#[test]
fn test_string_indexing() {
    let output = run_xe(
        r#"
s = "hello"
first = s[0]
last = s[4]
print(first)
print(last)
if s[0] == "h":
    print("matches h")
"#,
    )
    .unwrap();

    assert_eq!(output.trim(), "h\no\nmatches h");
}

#[test]
fn test_nested_list_iteration() {
    let output = run_xe(
        r#"
matrix = [[1.0, 2.0], [3.0, 4.0]]
for row in matrix:
    print(row)
"#,
    )
    .unwrap();

    assert_eq!(output.trim(), "[1, 2]\n[3, 4]");
}

#[test]
fn test_native_list_out_of_bounds_runtime_error() {
    let result = run_xe(
        r#"
items = [1.0, 2.0]
print(items[5])
"#,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Runtime error: list index 5 out of bounds"));
}

#[test]
fn test_for_loop_variable_shadowing_global() {
    let output = run_xe(
        r#"
i = 0
for i in [10, 20]:
    print(i)
print("after:", i)
"#,
    )
    .unwrap();

    assert_eq!(output.trim(), "10\n20\nafter: 0");
}

#[test]
fn test_function_param_shadowing_global() {
    let output = run_xe(
        r#"
x = 100

fun foo(x):
    print("inside:", x)

foo(42)
print("outside:", x)
"#,
    )
    .unwrap();

    assert_eq!(output.trim(), "inside: 42\noutside: 100");
}

#[test]
fn test_mixed_indentation_spaces_and_tabs() {
    let output = run_xe(
        "if true:\n \t \tx = 42\n \t \tprint(x)\n",
    )
    .unwrap();

    assert_eq!(output.trim(), "42");
}

#[test]
fn test_compile_does_not_destroy_existing_rs_file() {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_safety_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let original_content = "// Original precious code that must not be deleted";
    let protected_rs_file = temp_dir.join("program.rs");
    fs::write(&protected_rs_file, original_content).unwrap();

    let xe_file = temp_dir.join("main.xe");
    fs::write(&xe_file, "print(\"hello from xe\")\n").unwrap();

    let binary_output = temp_dir.join("program");

    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("compile")
        .arg(&xe_file)
        .arg("-o")
        .arg(&binary_output)
        .output()
        .unwrap();

    assert!(output.status.success());
    // Verify protected_rs_file still exists with its original content!
    assert!(protected_rs_file.exists());
    let current_content = fs::read_to_string(&protected_rs_file).unwrap();
    assert_eq!(current_content, original_content);

    let _ = fs::remove_dir_all(&temp_dir);
}

