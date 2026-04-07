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
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let xe_file = temp_dir.join("input.xe");
    fs::write(&xe_file, source).map_err(|e| e.to_string())?;

    let output = Command::new(env!("CARGO_BIN_EXE_xe"))
        .arg("run")
        .arg(&xe_file)
        .output()
        .map_err(|e| e.to_string())?;

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
function double(n):
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
    assert!(rust_code.contains("xe_print"));
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
    assert!(result
        .unwrap_err()
        .contains("Runtime error: division by zero"));
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
    assert!(result
        .unwrap_err()
        .contains("Runtime error: repeat loop count expected a non-negative integer"));
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
    assert!(result
        .unwrap_err()
        .contains("Runtime error: list index 5 out of bounds"));
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
    assert!(result
        .unwrap_err()
        .contains("Runtime error: for-loop iteration expected text or list, got number"));
}

#[test]
fn test_runtime_error_for_invalid_arithmetic_types() {
    let result = run_xe(r#"print([1, 2] - 1)"#);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Runtime error: operator '-' expected a number, got list"));
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
function answer():
    return 1

function answer():
    return 2
"#,
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("function 'answer' is already defined"));
}

#[test]
fn test_function_scope_does_not_capture_outer_variables() {
    let result = run_xe(
        r#"
x = 1

function show():
    print(x)

show()
"#,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined variable 'x'"));
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
