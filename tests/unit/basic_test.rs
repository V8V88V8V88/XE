use std::process::Command;
use std::fs;
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
    let output = run_xe(r#"
x = 5
y = 10
print(x + y)
"#).unwrap();
    assert_eq!(output.trim(), "15");
}

#[test]
fn test_string_concatenation() {
    let output = run_xe(r#"print("Hello" + " " + "World")"#).unwrap();
    assert_eq!(output.trim(), "Hello World");
}

#[test]
fn test_if_statement() {
    let output = run_xe(r#"
x = 10
if x > 5:
    print("big")
else:
    print("small")
"#).unwrap();
    assert_eq!(output.trim(), "big");

    let output = run_xe(r#"
x = 3
if x > 5:
    print("big")
else:
    print("small")
"#).unwrap();
    assert_eq!(output.trim(), "small");
}

#[test]
fn test_repeat_loop() {
    let output = run_xe(r#"
repeat 3 times:
    print("hi")
"#).unwrap();
    assert_eq!(output.trim(), "hi\nhi\nhi");
}

#[test]
fn test_function_definition() {
    let output = run_xe(r#"
function double(n):
    return n * 2

print(double(21))
"#).unwrap();
    assert_eq!(output.trim(), "42");
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
    let output = run_xe(r#"
items = [1, 2, 3]
print(length(items))
"#).unwrap();
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