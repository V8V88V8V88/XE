// XE runtime. This file is copied verbatim to the top of every compiled XE program;
// the generated code after it defines `XE_LOCATIONS` and calls `xe_start`.
#![allow(
    dead_code,
    unused_mut,
    unused_variables,
    unused_assignments,
    unused_imports,
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    unused_parens,
    unused_braces,
    unreachable_code,
    unreachable_patterns,
    clippy::all
)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// A hashable map key: numbers, text and booleans can be keys.
#[derive(Clone, PartialEq, Eq, Hash)]
enum XeKey {
    Number(u64),
    Text(String),
    Boolean(bool),
}

impl XeKey {
    fn from_value(value: &XeValue) -> XeKey {
        match value {
            XeValue::Number(n) => {
                if n.is_nan() {
                    xe_runtime_error("NaN cannot be used as a map key");
                }
                // 0 and -0 are the same key.
                let n = if *n == 0.0 { 0.0 } else { *n };
                XeKey::Number(n.to_bits())
            }
            XeValue::Text(s) => XeKey::Text(s.clone()),
            XeValue::Boolean(b) => XeKey::Boolean(*b),
            other => xe_runtime_error(&format!(
                "a {} cannot be used as a map key; use a number, text or boolean",
                other.type_name()
            )),
        }
    }

    fn to_value(&self) -> XeValue {
        match self {
            XeKey::Number(bits) => XeValue::Number(f64::from_bits(*bits)),
            XeKey::Text(s) => XeValue::Text(s.clone()),
            XeKey::Boolean(b) => XeValue::Boolean(*b),
        }
    }
}

impl fmt::Display for XeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", XeItem(&self.to_value()))
    }
}

/// A value shown inside a list, map or struct: text is quoted so `"1"` and `1` differ.
struct XeItem<'a>(&'a XeValue);

impl fmt::Display for XeItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            XeValue::Text(s) => write!(f, "{:?}", s),
            other => write!(f, "{}", other),
        }
    }
}

/// An insertion-ordered map.
#[derive(Clone, Default)]
struct XeMap {
    entries: Vec<(XeKey, XeValue)>,
    index: HashMap<XeKey, usize>,
}

impl XeMap {
    fn get(&self, key: &XeKey) -> Option<&XeValue> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    fn insert(&mut self, key: XeKey, value: XeValue) {
        match self.index.get(&key) {
            Some(&i) => self.entries[i].1 = value,
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
            }
        }
    }

    fn remove(&mut self, key: &XeKey) -> Option<XeValue> {
        let i = self.index.remove(key)?;
        let (_, value) = self.entries.remove(i);
        for position in self.index.values_mut() {
            if *position > i {
                *position -= 1;
            }
        }
        Some(value)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Clone)]
struct XeStruct {
    name: String,
    fields: Vec<(String, XeValue)>,
}

impl XeStruct {
    fn get(&self, field: &str) -> Option<&XeValue> {
        self.fields.iter().find(|(name, _)| name == field).map(|(_, v)| v)
    }

    fn get_mut(&mut self, field: &str) -> Option<&mut XeValue> {
        self.fields
            .iter_mut()
            .find(|(name, _)| name == field)
            .map(|(_, v)| v)
    }
}

struct XeFunction {
    name: String,
    min_args: usize,
    max_args: Option<usize>,
    call: Box<dyn Fn(Vec<XeValue>) -> XeValue>,
}

/// A dynamic XE value. Lists, maps, structs and functions are shared references:
/// assigning or passing them never copies, as in Python.
#[derive(Clone)]
enum XeValue {
    None,
    Number(f64),
    Text(String),
    Boolean(bool),
    List(Rc<RefCell<Vec<XeValue>>>),
    Map(Rc<RefCell<XeMap>>),
    Struct(Rc<RefCell<XeStruct>>),
    Function(Rc<XeFunction>),
}

impl From<f64> for XeValue {
    fn from(n: f64) -> Self {
        XeValue::Number(n)
    }
}

impl From<bool> for XeValue {
    fn from(b: bool) -> Self {
        XeValue::Boolean(b)
    }
}

impl From<String> for XeValue {
    fn from(s: String) -> Self {
        XeValue::Text(s)
    }
}

impl From<&str> for XeValue {
    fn from(s: &str) -> Self {
        XeValue::Text(s.to_string())
    }
}

impl XeValue {
    fn type_name(&self) -> &'static str {
        match self {
            XeValue::None => "none",
            XeValue::Number(_) => "number",
            XeValue::Text(_) => "text",
            XeValue::Boolean(_) => "boolean",
            XeValue::List(_) => "list",
            XeValue::Map(_) => "map",
            XeValue::Struct(_) => "struct",
            XeValue::Function(_) => "function",
        }
    }

    fn as_f64(&self) -> f64 {
        match self {
            XeValue::Number(n) => *n,
            other => xe_runtime_error(&format!("expected a number, got {}", other.type_name())),
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            XeValue::Boolean(b) => *b,
            other => xe_runtime_error(&format!("expected a boolean, got {}", other.type_name())),
        }
    }

    fn as_string(&self) -> String {
        match self {
            XeValue::Text(s) => s.clone(),
            other => xe_runtime_error(&format!("expected text, got {}", other.type_name())),
        }
    }

    fn list(&self, context: &str) -> Rc<RefCell<Vec<XeValue>>> {
        match self {
            XeValue::List(items) => items.clone(),
            other => xe_runtime_error(&format!(
                "{} expected a list, got {}",
                context,
                other.type_name()
            )),
        }
    }
}

fn xe_list(items: Vec<XeValue>) -> XeValue {
    XeValue::List(Rc::new(RefCell::new(items)))
}

fn xe_make_map(pairs: Vec<(XeValue, XeValue)>) -> XeValue {
    let mut map = XeMap::default();
    for (key, value) in pairs {
        map.insert(XeKey::from_value(&key), value);
    }
    XeValue::Map(Rc::new(RefCell::new(map)))
}

fn xe_make_struct(name: &str, field_names: &[&str], values: Vec<XeValue>) -> XeValue {
    let fields = field_names
        .iter()
        .zip(values)
        .map(|(name, value)| (name.to_string(), value))
        .collect();
    XeValue::Struct(Rc::new(RefCell::new(XeStruct {
        name: name.to_string(),
        fields,
    })))
}

fn xe_function(
    name: &str,
    min_args: usize,
    max_args: Option<usize>,
    call: impl Fn(Vec<XeValue>) -> XeValue + 'static,
) -> XeValue {
    XeValue::Function(Rc::new(XeFunction {
        name: name.to_string(),
        min_args,
        max_args,
        call: Box::new(call),
    }))
}

fn xe_call(callee: &XeValue, args: Vec<XeValue>) -> XeValue {
    match callee {
        XeValue::Function(function) => {
            let count = args.len();
            let too_few = count < function.min_args;
            let too_many = function.max_args.is_some_and(|max| count > max);
            if too_few || too_many {
                let expected = match function.max_args {
                    Some(max) if max == function.min_args => max.to_string(),
                    Some(max) => format!("{} to {}", function.min_args, max),
                    None => format!("at least {}", function.min_args),
                };
                xe_runtime_error(&format!(
                    "function '{}' expects {} arguments, got {}",
                    function.name, expected, count
                ));
            }
            (function.call)(args)
        }
        other => xe_runtime_error(&format!("a {} value is not a function", other.type_name())),
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

thread_local! {
    /// Containers being displayed, so self-containing values print `[...]`.
    static XE_DISPLAYING: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

fn xe_display_once(
    f: &mut fmt::Formatter<'_>,
    address: usize,
    placeholder: &str,
    body: impl FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
    if XE_DISPLAYING.with(|seen| seen.borrow().contains(&address)) {
        return write!(f, "{}", placeholder);
    }
    XE_DISPLAYING.with(|seen| seen.borrow_mut().push(address));
    let result = body(f);
    XE_DISPLAYING.with(|seen| seen.borrow_mut().pop());
    result
}

fn xe_format_number(n: f64) -> String {
    if n == 0.0 {
        // Avoid printing negative zero as "-0".
        "0".to_string()
    } else {
        n.to_string()
    }
}

impl fmt::Display for XeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XeValue::None => write!(f, "none"),
            XeValue::Number(n) => write!(f, "{}", xe_format_number(*n)),
            XeValue::Text(s) => write!(f, "{}", s),
            XeValue::Boolean(b) => write!(f, "{}", b),
            XeValue::List(items) => xe_display_once(f, Rc::as_ptr(items) as usize, "[...]", |f| {
                write!(f, "[")?;
                for (i, item) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", XeItem(item))?;
                }
                write!(f, "]")
            }),
            XeValue::Map(map) => xe_display_once(f, Rc::as_ptr(map) as usize, "{...}", |f| {
                write!(f, "{{")?;
                for (i, (key, value)) in map.borrow().entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, XeItem(value))?;
                }
                write!(f, "}}")
            }),
            XeValue::Struct(object) => {
                xe_display_once(f, Rc::as_ptr(object) as usize, "...", |f| {
                    let object = object.borrow();
                    write!(f, "{} {{ ", object.name)?;
                    for (i, (name, value)) in object.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", name, XeItem(value))?;
                    }
                    write!(f, " }}")
                })
            }
            XeValue::Function(function) => write!(f, "<function {}>", function.name),
        }
    }
}

// ---------------------------------------------------------------------------
// Errors, locations and program start
// ---------------------------------------------------------------------------

/// The payload of an XE runtime error; `try`/`catch` catches it.
struct XeRuntimeError(String);

fn xe_runtime_error(message: &str) -> ! {
    std::panic::panic_any(XeRuntimeError(message.to_string()))
}

fn xe_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(error) = payload.downcast_ref::<XeRuntimeError>() {
        error.0.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        format!("internal error: {}", message)
    } else if let Some(message) = payload.downcast_ref::<String>() {
        format!("internal error: {}", message)
    } else {
        "internal error".to_string()
    }
}

/// Control flow leaving a `try` body, which runs inside a closure.
enum XeFlow<R> {
    Normal,
    Break,
    Continue,
    Return(R),
}

/// Index into `XE_LOCATIONS` of the statement currently executing.
static XE_LOCATION: AtomicUsize = AtomicUsize::new(0);

#[inline(always)]
fn xe_loc(id: usize) {
    XE_LOCATION.store(id, Ordering::Relaxed);
}

fn xe_start(program: fn()) {
    // Errors are reported below, with the XE source location, instead of by Rust.
    std::panic::set_hook(Box::new(|_| {}));
    // A large stack so deeply recursive XE programs work.
    let outcome = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || std::panic::catch_unwind(program))
        .map(|handle| handle.join().unwrap_or_else(Err))
        .unwrap_or_else(|_| std::panic::catch_unwind(program));
    let _ = io::stdout().flush();
    if let Err(payload) = outcome {
        let message = xe_panic_message(payload);
        match XE_LOCATIONS.get(XE_LOCATION.load(Ordering::Relaxed)) {
            Some((file, line, text)) if *line > 0 => {
                eprintln!("Runtime error in {} at line {}: {}", file, line, message);
                eprintln!("    {}", text.trim());
            }
            _ => eprintln!("Runtime error: {}", message),
        }
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Module-level variables
// ---------------------------------------------------------------------------

type XeGlobal<T> = std::thread::LocalKey<RefCell<Option<T>>>;

fn xe_gget<T: Clone + 'static>(key: &'static XeGlobal<T>, name: &str) -> T {
    key.with(|cell| match &*cell.borrow() {
        Some(value) => value.clone(),
        None => xe_runtime_error(&format!("variable '{}' was used before it was assigned", name)),
    })
}

fn xe_gset<T: 'static>(key: &'static XeGlobal<T>, value: T) {
    key.with(|cell| *cell.borrow_mut() = Some(value));
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

fn xe_div(left: f64, right: f64) -> f64 {
    if right == 0.0 {
        xe_runtime_error("division by zero");
    }
    left / right
}

fn xe_floor_div(left: f64, right: f64) -> f64 {
    if right == 0.0 {
        xe_runtime_error("division by zero");
    }
    (left / right).floor()
}

/// Modulo with the sign of the divisor, so `a == (a // b) * b + a % b`.
fn xe_mod(left: f64, right: f64) -> f64 {
    if right == 0.0 {
        xe_runtime_error("modulo by zero");
    }
    let remainder = left % right;
    if remainder != 0.0 && ((remainder < 0.0) != (right < 0.0)) {
        remainder + right
    } else {
        remainder
    }
}

fn xe_pow(left: f64, right: f64) -> f64 {
    left.powf(right)
}

fn xe_concat(mut left: String, right: &str) -> String {
    left.push_str(right);
    left
}

fn xe_operator_error(op: &str, left: &XeValue, right: &XeValue) -> ! {
    xe_runtime_error(&format!(
        "operator '{}' is not defined for {} and {}",
        op,
        left.type_name(),
        right.type_name()
    ))
}

fn xe_add(left: XeValue, right: XeValue) -> XeValue {
    match (&left, &right) {
        (XeValue::Number(a), XeValue::Number(b)) => XeValue::Number(a + b),
        (XeValue::Text(a), _) => XeValue::Text(format!("{}{}", a, right)),
        (_, XeValue::Text(b)) => XeValue::Text(format!("{}{}", left, b)),
        (XeValue::List(a), XeValue::List(b)) => {
            let mut items = a.borrow().clone();
            items.extend(b.borrow().iter().cloned());
            xe_list(items)
        }
        _ => xe_operator_error("+", &left, &right),
    }
}

/// A binary arithmetic operator on dynamic values, used by `x[i] op= v` and `x.f op= v`.
fn xe_binary(op: &str, left: XeValue, right: XeValue) -> XeValue {
    if op == "+" {
        return xe_add(left, right);
    }
    let (a, b) = match (&left, &right) {
        (XeValue::Number(a), XeValue::Number(b)) => (*a, *b),
        _ => xe_operator_error(op, &left, &right),
    };
    XeValue::Number(match op {
        "-" => a - b,
        "*" => a * b,
        "/" => xe_div(a, b),
        "//" => xe_floor_div(a, b),
        "%" => xe_mod(a, b),
        "**" => xe_pow(a, b),
        _ => xe_operator_error(op, &left, &right),
    })
}

/// Numbers within a tiny relative tolerance compare equal, so `0.1 + 0.2 == 0.3`.
fn xe_num_eq(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

fn xe_eq(left: &XeValue, right: &XeValue) -> bool {
    match (left, right) {
        (XeValue::None, XeValue::None) => true,
        (XeValue::Number(a), XeValue::Number(b)) => xe_num_eq(*a, *b),
        (XeValue::Text(a), XeValue::Text(b)) => a == b,
        (XeValue::Boolean(a), XeValue::Boolean(b)) => a == b,
        (XeValue::List(a), XeValue::List(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let (a, b) = (a.borrow(), b.borrow());
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| xe_eq(x, y))
        }
        (XeValue::Map(a), XeValue::Map(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let (a, b) = (a.borrow(), b.borrow());
            a.len() == b.len()
                && a.entries
                    .iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| xe_eq(value, other)))
        }
        (XeValue::Struct(a), XeValue::Struct(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let (a, b) = (a.borrow(), b.borrow());
            a.name == b.name
                && a.fields.len() == b.fields.len()
                && a.fields
                    .iter()
                    .all(|(name, value)| b.get(name).is_some_and(|other| xe_eq(value, other)))
        }
        (XeValue::Function(a), XeValue::Function(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn xe_ordering(left: &XeValue, right: &XeValue, op: &str) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (XeValue::Number(a), XeValue::Number(b)) => a.partial_cmp(b),
        (XeValue::Text(a), XeValue::Text(b)) => Some(a.cmp(b)),
        _ => xe_operator_error(op, left, right),
    }
}

fn xe_compare(left: &XeValue, right: &XeValue, op: &str) -> bool {
    use std::cmp::Ordering::*;
    match xe_ordering(left, right, op) {
        // NaN compares false with everything.
        None => false,
        Some(order) => match op {
            "<" => order == Less,
            ">" => order == Greater,
            "<=" => order != Greater,
            _ => order != Less,
        },
    }
}

fn xe_truthy(value: &XeValue) -> bool {
    match value {
        XeValue::None => false,
        XeValue::Number(n) => *n != 0.0,
        XeValue::Text(s) => !s.is_empty(),
        XeValue::Boolean(b) => *b,
        XeValue::List(items) => !items.borrow().is_empty(),
        XeValue::Map(map) => map.borrow().len() > 0,
        XeValue::Struct(_) | XeValue::Function(_) => true,
    }
}

fn xe_in(item: &XeValue, container: &XeValue) -> bool {
    xe_builtin_contains(container, item)
}

// ---------------------------------------------------------------------------
// Indexing, slicing and fields
// ---------------------------------------------------------------------------

fn xe_whole_number(value: f64, what: &str) -> i64 {
    if !value.is_finite() || value.fract() != 0.0 {
        xe_runtime_error(&format!("{} must be a whole number, got {}", what, xe_format_number(value)));
    }
    value as i64
}

/// The position for index `index` (negative counts from the end) in a sequence of `len`.
fn xe_position(len: usize, index: f64, what: &str) -> usize {
    let raw = xe_whole_number(index, &format!("{} index", what));
    let position = if raw < 0 { raw + len as i64 } else { raw };
    if position < 0 || position >= len as i64 {
        xe_runtime_error(&format!(
            "{} index {} out of bounds (length {})",
            what, raw, len
        ));
    }
    position as usize
}

fn xe_index_number(index: &XeValue, what: &str) -> f64 {
    match index {
        XeValue::Number(n) => *n,
        other => xe_runtime_error(&format!(
            "{} index must be a number, got {}",
            what,
            other.type_name()
        )),
    }
}

fn xe_index(object: &XeValue, index: &XeValue) -> XeValue {
    match object {
        XeValue::List(items) => {
            let items = items.borrow();
            let position = xe_position(items.len(), xe_index_number(index, "list"), "list");
            items[position].clone()
        }
        XeValue::Text(text) => {
            let chars: Vec<char> = text.chars().collect();
            let position = xe_position(chars.len(), xe_index_number(index, "text"), "text");
            XeValue::Text(chars[position].to_string())
        }
        XeValue::Map(map) => {
            let key = XeKey::from_value(index);
            map.borrow()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| xe_runtime_error(&format!("key {} not found in map", key)))
        }
        XeValue::Struct(object) => {
            let field = index.to_string();
            let object = object.borrow();
            object.get(&field).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("struct '{}' has no field '{}'", object.name, field))
            })
        }
        other => xe_runtime_error(&format!("cannot index into a {} value", other.type_name())),
    }
}

fn xe_set_index(object: &XeValue, index: XeValue, value: XeValue) {
    match object {
        XeValue::List(items) => {
            let mut items = items.borrow_mut();
            let position = xe_position(items.len(), xe_index_number(&index, "list"), "list");
            items[position] = value;
        }
        XeValue::Map(map) => map.borrow_mut().insert(XeKey::from_value(&index), value),
        XeValue::Struct(_) => xe_set_field(object, &index.to_string(), value),
        XeValue::Text(_) => {
            xe_runtime_error("text cannot be changed in place; build a new text instead")
        }
        other => xe_runtime_error(&format!(
            "cannot assign to an element of a {} value",
            other.type_name()
        )),
    }
}

fn xe_get_field(object: &XeValue, field: &str) -> XeValue {
    match object {
        XeValue::Struct(object) => {
            let object = object.borrow();
            object.get(field).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("struct '{}' has no field '{}'", object.name, field))
            })
        }
        XeValue::Map(map) => map
            .borrow()
            .get(&XeKey::Text(field.to_string()))
            .cloned()
            .unwrap_or_else(|| xe_runtime_error(&format!("map has no key \"{}\"", field))),
        other => xe_runtime_error(&format!(
            "cannot access field '{}' on a {} value",
            field,
            other.type_name()
        )),
    }
}

fn xe_set_field(object: &XeValue, field: &str, value: XeValue) {
    match object {
        XeValue::Struct(object) => {
            let mut object = object.borrow_mut();
            let name = object.name.clone();
            match object.get_mut(field) {
                Some(slot) => *slot = value,
                None => xe_runtime_error(&format!("struct '{}' has no field '{}'", name, field)),
            }
        }
        XeValue::Map(map) => map.borrow_mut().insert(XeKey::Text(field.to_string()), value),
        other => xe_runtime_error(&format!(
            "cannot set field '{}' on a {} value",
            field,
            other.type_name()
        )),
    }
}

/// Python-style slice bounds: negative values count from the end and are clamped.
fn xe_slice_range(len: usize, start: Option<f64>, end: Option<f64>) -> (usize, usize) {
    let clamp = |bound: f64, what: &str| {
        let raw = xe_whole_number(bound, what);
        let position = if raw < 0 { raw + len as i64 } else { raw };
        position.clamp(0, len as i64) as usize
    };
    let start = start.map_or(0, |s| clamp(s, "slice start"));
    let end = end.map_or(len, |e| clamp(e, "slice end"));
    (start, end.max(start))
}

fn xe_slice(object: &XeValue, start: Option<f64>, end: Option<f64>) -> XeValue {
    match object {
        XeValue::List(items) => {
            let items = items.borrow();
            let (start, end) = xe_slice_range(items.len(), start, end);
            xe_list(items[start..end].to_vec())
        }
        XeValue::Text(text) => {
            let chars: Vec<char> = text.chars().collect();
            let (start, end) = xe_slice_range(chars.len(), start, end);
            XeValue::Text(chars[start..end].iter().collect())
        }
        other => xe_runtime_error(&format!("cannot slice a {} value", other.type_name())),
    }
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

/// Values a `for` loop visits: list items, text characters, map keys or struct field names.
fn xe_iter(value: &XeValue) -> Vec<XeValue> {
    match value {
        XeValue::List(items) => items.borrow().clone(),
        XeValue::Text(s) => s.chars().map(|c| XeValue::Text(c.to_string())).collect(),
        XeValue::Map(map) => map.borrow().entries.iter().map(|(k, _)| k.to_value()).collect(),
        XeValue::Struct(object) => object
            .borrow()
            .fields
            .iter()
            .map(|(name, _)| XeValue::Text(name.clone()))
            .collect(),
        other => xe_runtime_error(&format!("cannot loop over a {} value", other.type_name())),
    }
}

struct XeRange {
    next: f64,
    stop: f64,
    step: f64,
}

impl Iterator for XeRange {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        let more = if self.step > 0.0 {
            self.next < self.stop
        } else {
            self.next > self.stop
        };
        if !more {
            return None;
        }
        let value = self.next;
        self.next += self.step;
        Some(value)
    }
}

fn xe_range(start: f64, stop: f64, step: f64) -> XeRange {
    if !start.is_finite() || !stop.is_finite() || !step.is_finite() {
        xe_runtime_error("range() arguments must be finite numbers");
    }
    if step == 0.0 {
        xe_runtime_error("range() step cannot be zero");
    }
    XeRange {
        next: start,
        stop,
        step,
    }
}

fn xe_repeat_count(count: f64) -> u64 {
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
        xe_runtime_error(&format!(
            "repeat count must be a non-negative whole number, got {}",
            xe_format_number(count)
        ));
    }
    count as u64
}

// ---------------------------------------------------------------------------
// Built-in functions
// ---------------------------------------------------------------------------

fn xe_builtin_print(args: Vec<XeValue>) {
    let line = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(" ");
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", line);
}

fn xe_builtin_input(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().lock().read_line(&mut input);
    input.trim().to_string()
}

fn xe_builtin_length(value: &XeValue) -> f64 {
    match value {
        XeValue::Text(s) => s.chars().count() as f64,
        XeValue::List(items) => items.borrow().len() as f64,
        XeValue::Map(map) => map.borrow().len() as f64,
        XeValue::Struct(object) => object.borrow().fields.len() as f64,
        other => xe_runtime_error(&format!(
            "length() expected text, list, map, or struct, got {}",
            other.type_name()
        )),
    }
}

fn xe_builtin_type(value: &XeValue) -> String {
    value.type_name().to_string()
}

fn xe_builtin_convert(value: &XeValue, target: &str) -> XeValue {
    match target {
        "number" => match value {
            XeValue::Number(n) => XeValue::Number(*n),
            XeValue::Text(s) => match s.trim().parse::<f64>() {
                Ok(n) => XeValue::Number(n),
                Err(_) => xe_runtime_error(&format!("cannot convert text '{}' to number", s)),
            },
            XeValue::Boolean(b) => XeValue::Number(if *b { 1.0 } else { 0.0 }),
            other => xe_runtime_error(&format!("cannot convert {} to number", other.type_name())),
        },
        "text" => XeValue::Text(value.to_string()),
        "boolean" => XeValue::Boolean(xe_truthy(value)),
        _ => xe_runtime_error(&format!("unsupported convert() target '{}'", target)),
    }
}

fn xe_builtin_append(list: &XeValue, item: XeValue) -> XeValue {
    list.list("append()").borrow_mut().push(item);
    list.clone()
}

fn xe_builtin_pop(list: &XeValue, index: Option<f64>) -> XeValue {
    let items = list.list("pop()");
    let mut items = items.borrow_mut();
    if items.is_empty() {
        xe_runtime_error("pop() called on an empty list");
    }
    let position = match index {
        Some(index) => xe_position(items.len(), index, "list"),
        None => items.len() - 1,
    };
    items.remove(position)
}

fn xe_builtin_insert(list: &XeValue, index: f64, item: XeValue) {
    let items = list.list("insert()");
    let mut items = items.borrow_mut();
    let len = items.len() as i64;
    let raw = xe_whole_number(index, "insert() index");
    let position = if raw < 0 { raw + len } else { raw }.clamp(0, len) as usize;
    items.insert(position, item);
}

fn xe_builtin_remove(target: &XeValue, key: &XeValue) -> XeValue {
    match target {
        XeValue::Map(map) => {
            let key = XeKey::from_value(key);
            map.borrow_mut()
                .remove(&key)
                .unwrap_or_else(|| xe_runtime_error(&format!("key {} not found in map", key)))
        }
        XeValue::List(items) => {
            let mut items = items.borrow_mut();
            match items.iter().position(|item| xe_eq(item, key)) {
                Some(position) => items.remove(position),
                None => xe_runtime_error(&format!("remove(): {} is not in the list", key)),
            }
        }
        other => xe_runtime_error(&format!("remove() expected a map or list, got {}", other.type_name())),
    }
}

fn xe_builtin_keys(value: &XeValue) -> XeValue {
    match value {
        XeValue::Map(_) | XeValue::Struct(_) => xe_list(xe_iter(value)),
        other => xe_runtime_error(&format!("keys() expected a map or struct, got {}", other.type_name())),
    }
}

fn xe_builtin_values(value: &XeValue) -> XeValue {
    match value {
        XeValue::Map(map) => xe_list(map.borrow().entries.iter().map(|(_, v)| v.clone()).collect()),
        XeValue::Struct(object) => {
            xe_list(object.borrow().fields.iter().map(|(_, v)| v.clone()).collect())
        }
        other => xe_runtime_error(&format!("values() expected a map or struct, got {}", other.type_name())),
    }
}

fn xe_builtin_has_key(target: &XeValue, key: &XeValue) -> bool {
    match (target, key) {
        (XeValue::Map(map), XeValue::Number(_) | XeValue::Text(_) | XeValue::Boolean(_)) => {
            map.borrow().get(&XeKey::from_value(key)).is_some()
        }
        (XeValue::Struct(object), XeValue::Text(field)) => object.borrow().get(field).is_some(),
        _ => false,
    }
}

fn xe_builtin_contains(container: &XeValue, item: &XeValue) -> bool {
    match container {
        XeValue::List(items) => items.borrow().iter().any(|v| xe_eq(v, item)),
        XeValue::Text(s) => s.contains(&item.to_string()),
        XeValue::Map(_) | XeValue::Struct(_) => xe_builtin_has_key(container, item),
        other => xe_runtime_error(&format!(
            "contains() expected a list, text, map, or struct, got {}",
            other.type_name()
        )),
    }
}

fn xe_builtin_split(text: &str, separator: Option<&str>) -> XeValue {
    let parts: Vec<XeValue> = match separator {
        None => text.split_whitespace().map(XeValue::from).collect(),
        Some("") => xe_runtime_error("split() separator cannot be empty"),
        Some(separator) => text.split(separator).map(XeValue::from).collect(),
    };
    xe_list(parts)
}

fn xe_builtin_join(list: &XeValue, separator: &str) -> String {
    list.list("join()")
        .borrow()
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(separator)
}

fn xe_builtin_range(start: f64, stop: f64, step: f64) -> XeValue {
    xe_list(xe_range(start, stop, step).map(XeValue::Number).collect())
}

fn xe_builtin_upper(text: &str) -> String {
    text.to_uppercase()
}

fn xe_builtin_lower(text: &str) -> String {
    text.to_lowercase()
}

fn xe_builtin_trim(text: &str) -> String {
    text.trim().to_string()
}

fn xe_builtin_replace(text: &str, old: &str, new: &str) -> String {
    if old.is_empty() {
        xe_runtime_error("replace() cannot replace empty text");
    }
    text.replace(old, new)
}

fn xe_builtin_starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

fn xe_builtin_ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

/// The character position of the first occurrence of `needle`, or -1.
fn xe_builtin_find(text: &str, needle: &str) -> f64 {
    match text.find(needle) {
        Some(byte_index) => text[..byte_index].chars().count() as f64,
        None => -1.0,
    }
}

fn xe_builtin_abs(n: f64) -> f64 {
    n.abs()
}

fn xe_builtin_floor(n: f64) -> f64 {
    n.floor()
}

fn xe_builtin_ceil(n: f64) -> f64 {
    n.ceil()
}

fn xe_builtin_sqrt(n: f64) -> f64 {
    if n < 0.0 {
        xe_runtime_error(&format!("sqrt() of a negative number ({})", xe_format_number(n)));
    }
    n.sqrt()
}

/// Rounds half away from zero, optionally to a number of decimal places.
fn xe_builtin_round(n: f64, digits: Option<f64>) -> f64 {
    match digits {
        None => n.round(),
        Some(digits) => {
            let digits = xe_whole_number(digits, "round() digits");
            let factor = 10f64.powi(digits as i32);
            (n * factor).round() / factor
        }
    }
}

/// Arguments for min() and max(): several values, or a single list.
fn xe_extremum_items(args: Vec<XeValue>, name: &str) -> Vec<XeValue> {
    let items = match args.as_slice() {
        [XeValue::List(items)] => items.borrow().clone(),
        _ => args,
    };
    if items.is_empty() {
        xe_runtime_error(&format!("{}() of an empty list", name));
    }
    items
}

fn xe_builtin_min(args: Vec<XeValue>) -> XeValue {
    let items = xe_extremum_items(args, "min");
    let mut best = items[0].clone();
    for item in &items[1..] {
        if xe_compare(item, &best, "<") {
            best = item.clone();
        }
    }
    best
}

fn xe_builtin_max(args: Vec<XeValue>) -> XeValue {
    let items = xe_extremum_items(args, "max");
    let mut best = items[0].clone();
    for item in &items[1..] {
        if xe_compare(item, &best, ">") {
            best = item.clone();
        }
    }
    best
}

fn xe_builtin_sum(list: &XeValue) -> f64 {
    list.list("sum()")
        .borrow()
        .iter()
        .map(|item| match item {
            XeValue::Number(n) => *n,
            other => xe_runtime_error(&format!("sum() expected numbers, got {}", other.type_name())),
        })
        .sum()
}

/// Sorts a list in place and returns it; the items must be all numbers or all text.
fn xe_builtin_sort(list: &XeValue) -> XeValue {
    let items = list.list("sort()");
    let mut failed = None;
    items.borrow_mut().sort_by(|a, b| match xe_sort_order(a, b) {
        Some(order) => order,
        None => {
            failed.get_or_insert((a.type_name(), b.type_name()));
            std::cmp::Ordering::Equal
        }
    });
    if let Some((a, b)) = failed {
        xe_runtime_error(&format!("sort() cannot compare {} and {}", a, b));
    }
    list.clone()
}

fn xe_sort_order(a: &XeValue, b: &XeValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (XeValue::Number(x), XeValue::Number(y)) => Some(x.total_cmp(y)),
        (XeValue::Text(x), XeValue::Text(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// Reverses a list in place and returns it.
fn xe_builtin_reverse(list: &XeValue) -> XeValue {
    list.list("reverse()").borrow_mut().reverse();
    list.clone()
}

/// A shallow copy of a list, map or struct; other values are returned as they are.
fn xe_builtin_copy(value: &XeValue) -> XeValue {
    match value {
        XeValue::List(items) => xe_list(items.borrow().clone()),
        XeValue::Map(map) => XeValue::Map(Rc::new(RefCell::new(map.borrow().clone()))),
        XeValue::Struct(object) => XeValue::Struct(Rc::new(RefCell::new(object.borrow().clone()))),
        other => other.clone(),
    }
}

thread_local! {
    static XE_RANDOM_STATE: RefCell<u64> = RefCell::new({
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        (nanos ^ (std::process::id() as u64).rotate_left(32)) | 1
    });
}

/// A pseudo-random number in [0, 1) (xorshift64*; not for cryptography).
fn xe_builtin_random() -> f64 {
    XE_RANDOM_STATE.with(|state| {
        let mut x = *state.borrow();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *state.borrow_mut() = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    })
}

fn xe_builtin_args() -> XeValue {
    xe_list(std::env::args().skip(1).map(XeValue::Text).collect())
}

fn xe_builtin_exit(code: Option<f64>) {
    let code = code.map_or(0, |c| xe_whole_number(c, "exit code") as i32);
    let _ = io::stdout().flush();
    std::process::exit(code);
}

fn xe_builtin_read_file(path: &str) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| xe_runtime_error(&format!("cannot read file '{}': {}", path, e)))
}

fn xe_builtin_write_file(path: &str, text: &str) {
    if let Err(e) = std::fs::write(path, text) {
        xe_runtime_error(&format!("cannot write file '{}': {}", path, e));
    }
}

fn xe_builtin_append_file(path: &str, text: &str) {
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()));
    if let Err(e) = result {
        xe_runtime_error(&format!("cannot append to file '{}': {}", path, e));
    }
}

fn xe_builtin_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

fn xe_builtin_error(message: &XeValue) {
    xe_runtime_error(&message.to_string());
}
