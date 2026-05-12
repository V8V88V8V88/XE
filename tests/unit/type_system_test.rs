use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_unique_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn run_xe(source: &str) -> Result<String, String> {
    let id = get_unique_id();
    let temp_dir = std::env::temp_dir().join(format!("xe_test_sys_{}", id));
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
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[test]
fn test_native_list_optimization() {
    let result = run_xe(
        r#"
items = [1.0, 2.0, 3.0]
total = 0.0
for item in items:
    total = total + item
print(total)
"#,
    );
    assert_eq!(result.unwrap(), "6");
}

#[test]
fn test_mixed_list_fallback() {
    let result = run_xe(
        r#"
items = [1, "text", true]
print(items)
"#,
    );
    assert_eq!(result.unwrap(), "[1, text, true]");
}

#[test]
fn test_native_matrix_indexing() {
    let result = run_xe(
        r#"
matrix = [[1.0, 2.0], [3.0, 4.0]]
print(matrix[1][0])
"#,
    );
    assert_eq!(result.unwrap(), "3");
}

#[test]
fn test_native_function_inference_and_call() {
    let result = run_xe(
        r#"
fun double(n):
    return n * 2

fun quadruple(n):
    return double(double(n))

print(quadruple(5))
"#,
    );
    assert_eq!(result.unwrap(), "20");
}

#[test]
fn test_string_concatenation_with_coercion() {
    let result = run_xe(
        r#"
name = "World"
print("Hello " + name + "!")
"#,
    );
    assert_eq!(result.unwrap(), "Hello World!");
}

#[test]
fn test_large_numeric_loop_performance_correctness() {
    let result = run_xe(
        r#"
total = 0
repeat 100 times:
    total = total + 1
print(total)
"#,
    );
    assert_eq!(result.unwrap(), "100");
}
