# Ergonomic Rust Patterns

A catalog of transformations from clunky to clean. Each section shows the **before** (what you'll encounter in the wild) and **after** (what it should become). The after version is always equivalent in behavior and equal-or-better in performance.

## Table of Contents

1. [Iterator Chains vs Manual Loops](#1-iterator-chains-vs-manual-loops)
2. [Option and Result Combinators](#2-option-and-result-combinators)
3. [Let-Else and Early Returns](#3-let-else-and-early-returns)
4. [Type Conversions and From/Into](#4-type-conversions-and-frominto)
5. [String Handling](#5-string-handling)
6. [Pattern Matching Simplification](#6-pattern-matching-simplification)
7. [Builder and Constructor Patterns](#7-builder-and-constructor-patterns)
8. [Closure and Function Pointer Ergonomics](#8-closure-and-function-pointer-ergonomics)
9. [Smart Use of impl and Generics](#9-smart-use-of-impl-and-generics)
10. [#[inline] — When and Why](#10-inline--when-and-why)
11. [Derive and Default](#11-derive-and-default)
12. [Collection Operations](#12-collection-operations)
13. [Error Handling Design](#13-error-handling-design)
14. [Ownership and Borrowing Ergonomics](#14-ownership-and-borrowing-ergonomics)
15. [Trait Design for Usability](#15-trait-design-for-usability)
16. [Control Flow Simplification](#16-control-flow-simplification)

---

## 1. Iterator Chains vs Manual Loops

Manual loops with mutable accumulators are the single most common readability drain in Rust code. Iterator chains express the *what*, not the *how*.

**Before:**
```rust
let mut results = Vec::new();
for item in &items {
    if item.is_active() {
        results.push(item.name().to_uppercase());
    }
}
```

**After:**
```rust
let results: Vec<_> = items
    .iter()
    .filter(|item| item.is_active())
    .map(|item| item.name().to_uppercase())
    .collect();
```

The iterator version is also typically *faster* — the compiler can elide bounds checks and optimize the chain as a single pass.

**Before:**
```rust
let mut total = 0;
for entry in &ledger {
    if entry.category == Category::Revenue {
        total += entry.amount;
    }
}
```

**After:**
```rust
let total: i64 = ledger
    .iter()
    .filter(|e| e.category == Category::Revenue)
    .map(|e| e.amount)
    .sum();
```

**Before — nested loops with index tracking:**
```rust
let mut pairs = Vec::new();
for i in 0..items.len() {
    for j in (i + 1)..items.len() {
        if items[i].group == items[j].group {
            pairs.push((i, j));
        }
    }
}
```

**After:**
```rust
let pairs: Vec<_> = (0..items.len())
    .flat_map(|i| ((i + 1)..items.len()).map(move |j| (i, j)))
    .filter(|&(i, j)| items[i].group == items[j].group)
    .collect();
```

**When to keep the loop:** If the loop body has side effects, early breaks with complex conditions, or mutates multiple pieces of state, a `for` loop is often clearer. Don't force everything into an iterator chain.

---

## 2. Option and Result Combinators

Nested `match` and `if let` pyramids are the second biggest readability problem. Combinators flatten the logic.

**Before:**
```rust
fn get_user_email(id: u64) -> Option<String> {
    let user = match find_user(id) {
        Some(u) => u,
        None => return None,
    };
    let profile = match user.profile() {
        Some(p) => p,
        None => return None,
    };
    Some(profile.email.clone())
}
```

**After:**
```rust
fn get_user_email(id: u64) -> Option<String> {
    find_user(id)?
        .profile()
        .map(|p| p.email.clone())
}
```

**Before — unwrap_or with default:**
```rust
let name = match config.get("name") {
    Some(n) => n.clone(),
    None => String::from("default"),
};
```

**After:**
```rust
let name = config.get("name").cloned().unwrap_or_else(|| "default".into());
```

**Before — map + unwrap_or chain:**
```rust
let port: u16 = match env::var("PORT") {
    Ok(val) => match val.parse() {
        Ok(p) => p,
        Err(_) => 8080,
    },
    Err(_) => 8080,
};
```

**After:**
```rust
let port: u16 = env::var("PORT")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(8080);
```

**Key combinators to reach for:**
- `?` — propagate errors/None, always prefer over manual match-and-return
- `.map()` — transform the inner value
- `.and_then()` — chain fallible operations
- `.unwrap_or()` / `.unwrap_or_else()` — provide defaults
- `.ok()` — convert Result to Option (discard error)
- `.transpose()` — flip `Option<Result<T>>` to `Result<Option<T>>`

---

## 3. Let-Else and Early Returns

Guard clauses at the top of a function read like preconditions. They prevent the "arrow" anti-pattern (ever-deeper indentation).

**Before:**
```rust
fn process(input: Option<&str>) -> Result<Output, Error> {
    if let Some(text) = input {
        if !text.is_empty() {
            if let Ok(parsed) = parse(text) {
                return Ok(transform(parsed));
            } else {
                return Err(Error::Parse);
            }
        } else {
            return Err(Error::Empty);
        }
    } else {
        return Err(Error::Missing);
    }
}
```

**After:**
```rust
fn process(input: Option<&str>) -> Result<Output, Error> {
    let text = input.ok_or(Error::Missing)?;
    if text.is_empty() {
        return Err(Error::Empty);
    }
    let parsed = parse(text).map_err(|_| Error::Parse)?;
    Ok(transform(parsed))
}
```

**`let ... else` (Rust 1.65+) — for when you need the binding:**
```rust
fn process_record(data: &[u8]) -> Result<Record, Error> {
    let Some(header) = parse_header(data) else {
        return Err(Error::MissingHeader);
    };
    let Some(body) = parse_body(data, header.offset) else {
        return Err(Error::MissingBody);
    };
    Ok(Record { header, body })
}
```

---

## 4. Type Conversions and From/Into

Implement `From` on your types. It enables `into()` for free, works with `?` for error conversion, and makes APIs feel native.

**Before:**
```rust
fn create_user(name: &str) -> User {
    User {
        name: String::from(name),
        id: Uuid::new_v4(),
    }
}

// caller:
let user = create_user("alice");
```

**After — accept `impl Into<String>`:**
```rust
fn create_user(name: impl Into<String>) -> User {
    User {
        name: name.into(),
        id: Uuid::new_v4(),
    }
}

// caller can pass &str, String, Cow, SmolStr — whatever they have
let user = create_user("alice");
let user = create_user(existing_string);  // no .clone() needed
```

**Before — manual error conversion:**
```rust
fn load_config(path: &Path) -> Result<Config, AppError> {
    let data = fs::read_to_string(path).map_err(|e| AppError::Io(e))?;
    let config: Config = serde_json::from_str(&data).map_err(|e| AppError::Parse(e))?;
    Ok(config)
}
```

**After — implement From, let `?` do the work:**
```rust
impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self { AppError::Io(e) }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self { AppError::Parse(e) }
}

fn load_config(path: &Path) -> Result<Config, AppError> {
    let data = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&data)?;
    Ok(config)
}
```

Or use `thiserror` to derive it all.

---

## 5. String Handling

String types are where most Rust friction lives. Choose the right type and conversion path.

**Before:**
```rust
fn greet(name: &str) -> String {
    let mut s = String::new();
    s.push_str("Hello, ");
    s.push_str(name);
    s.push('!');
    s
}
```

**After:**
```rust
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

`format!` is not slow — it's the idiomatic way and the compiler optimizes it well.

**Before — unnecessary allocation:**
```rust
fn is_valid(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower == "yes" || lower == "true" || lower == "1"
}
```

**After:**
```rust
fn is_valid(input: &str) -> bool {
    input.eq_ignore_ascii_case("yes")
        || input.eq_ignore_ascii_case("true")
        || input == "1"
}
```

No allocation, handles the common case (ASCII) correctly.

**Before — `.to_string()` vs `.into()` vs `String::from()`:**

Use whichever is clearest in context, but be consistent within a file. Prefer:
- `"literal".to_owned()` or `"literal".into()` for string literals
- `.to_string()` when you actually want Display formatting
- `String::from()` is fine but verbose

**`Cow<str>` — when you might or might not need to allocate:**
```rust
fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}
```

This avoids allocation when the input is already clean — useful in hot paths.

---

## 6. Pattern Matching Simplification

**Before — matching on boolean-like enums:**
```rust
match self.state {
    State::Active => true,
    State::Inactive => false,
    State::Pending => false,
}
```

**After:**
```rust
matches!(self.state, State::Active)
```

**Before — single-variant extraction:**
```rust
let value = match result {
    Ok(v) => v,
    Err(e) => return Err(e),
};
```

**After:**
```rust
let value = result?;
```

**Before — matching to transform:**
```rust
let label = match status {
    Status::Ok => "ok",
    Status::Warn => "warning",
    Status::Error => "error",
};
```

This is already fine — `match` is the right tool here. Don't force this into a method unless it's used in multiple places.

**Before — nested matches:**
```rust
match outer {
    Some(inner) => match inner {
        Thing::A(val) => do_a(val),
        Thing::B(val) => do_b(val),
        _ => default(),
    },
    None => default(),
}
```

**After:**
```rust
match outer {
    Some(Thing::A(val)) => do_a(val),
    Some(Thing::B(val)) => do_b(val),
    _ => default(),
}
```

Rust match arms can destructure nested patterns in one level.

---

## 7. Builder and Constructor Patterns

**Before — many-argument constructor:**
```rust
let server = Server::new(
    "0.0.0.0",
    8080,
    true,
    None,
    Some(Duration::from_secs(30)),
    false,
);
```

Nobody can read this without checking the function signature. Use a builder or named fields.

**After — builder with method chaining:**
```rust
let server = Server::builder()
    .addr("0.0.0.0")
    .port(8080)
    .tls(true)
    .timeout(Duration::from_secs(30))
    .build();
```

**Simple alternative — struct literal with defaults:**
```rust
let server = Server {
    addr: "0.0.0.0".into(),
    port: 8080,
    tls: true,
    timeout: Some(Duration::from_secs(30)),
    ..Server::default()
};
```

This is often the best choice for internal types — no builder boilerplate needed.

**Tip:** If a struct has a `Default` impl and you only need to override a few fields, `..Default::default()` is perfectly idiomatic and zero-cost.

---

## 8. Closure and Function Pointer Ergonomics

**Before — closure that just calls a method:**
```rust
items.iter().map(|item| item.name()).collect::<Vec<_>>()
```

**After — if it's a simple method call, a closure is fine here.** Don't reach for `Item::name` unless it actually compiles and reads better. Method references in Rust are less ergonomic than in many languages.

**Before — closure that wraps a constructor:**
```rust
values.iter().map(|v| Some(*v)).collect::<Vec<_>>()
```

**After:**
```rust
values.iter().copied().map(Some).collect::<Vec<_>>()
```

`Some` is a function — you can pass it directly.

**Before — `|x| x` identity closure:**
```rust
results.into_iter().flat_map(|r| r).collect()
```

**After:**
```rust
results.into_iter().flatten().collect()
```

---

## 9. Smart Use of impl and Generics

**Before — overly generic signature:**
```rust
fn process<T: AsRef<str> + Into<String> + Clone + Debug>(input: T) -> String {
    let s: String = input.into();
    s.to_uppercase()
}
```

**After — just take what you need:**
```rust
fn process(input: &str) -> String {
    input.to_uppercase()
}
```

Generic bounds should earn their keep. If every caller passes a `&str`, just take a `&str`.

**`impl Trait` in argument position — when it genuinely helps:**
```rust
// Good: caller can pass any iterator, no need to collect first
fn sum_positives(values: impl Iterator<Item = i64>) -> i64 {
    values.filter(|&v| v > 0).sum()
}
```

**`impl Trait` in return position — hide complex types:**
```rust
// Good: hides the iterator chain type
fn active_users(users: &[User]) -> impl Iterator<Item = &User> {
    users.iter().filter(|u| u.is_active())
}
```

This is zero-cost — the compiler monomorphizes it. The caller gets a concrete iterator without the unreadable `Filter<Iter<...>>` type.

---

## 10. #[inline] — When and Why

`#[inline]` is a hint to the compiler that a function is worth inlining across crate boundaries. The compiler already inlines aggressively *within* a crate, so `#[inline]` on private functions in the same crate is usually pointless.

### When to use `#[inline]`

**Small public functions in library crates** — especially trivial accessors, constructors, and converters that are called from other crates:

```rust
#[inline]
pub fn is_empty(&self) -> bool {
    self.len == 0
}

#[inline]
pub fn as_str(&self) -> &str {
    &self.inner
}
```

Without `#[inline]`, these become real function calls across crate boundaries — a noticeable overhead for something that should compile to a single instruction.

**`#[inline(always)]`** — use sparingly. This forces inlining even when the compiler thinks it's a bad idea. Valid uses:
- Tiny functions in performance-critical paths that the compiler inexplicably refuses to inline (measure first)
- Functions in `#[no_std]` contexts where you need deterministic codegen

```rust
#[inline(always)]
pub fn wrapping_add(self, rhs: Self) -> Self {
    Self(self.0.wrapping_add(rhs.0))
}
```

### When NOT to use `#[inline]`

- **Private functions** in the same crate — the compiler already sees the body and will inline if beneficial
- **Large functions** — inlining a 50-line function bloats code size and can *hurt* performance (instruction cache pressure)
- **Functions that are rarely called** — startup code, error paths, configuration loading
- **Recursive functions** — the compiler can't inline infinite recursion
- **Async functions** — `#[inline]` on async fns does very little because the real work happens in the generated future

### The `#[inline]` decision checklist

1. Is it `pub` and called from another crate? → Consider `#[inline]`
2. Is the function body trivial (< 5 lines, no loops, no branches)? → Yes, `#[inline]`
3. Is it in a hot path that you've profiled? → `#[inline]`, possibly `#[inline(always)]`
4. Everything else → Leave it alone. Trust the compiler.

---

## 11. Derive and Default

**Before — manual Default impl that just uses literal defaults:**
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            retries: 0,
            timeout: Duration::from_secs(0),
            verbose: false,
            name: String::new(),
        }
    }
}
```

**After:**
```rust
#[derive(Default)]
struct Config {
    retries: u32,
    timeout: Duration,
    verbose: bool,
    name: String,
}
```

If every field's default is the type's `Default`, just derive it.

**When you need some non-default defaults** — only implement `Default` manually for the fields that differ:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            retries: 3,
            timeout: Duration::from_secs(30),
            verbose: false,
            name: String::new(),
        }
    }
}
```

This is fine — the manual impl is warranted because the defaults are domain-specific.

**Derive order convention:** `Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize` — not strictly required but widely followed. It helps readers predict what's derived.

---

## 12. Collection Operations

**Before — check-then-insert:**
```rust
if !map.contains_key(&key) {
    map.insert(key, compute_value());
}
let val = map.get(&key).unwrap();
```

**After:**
```rust
let val = map.entry(key).or_insert_with(|| compute_value());
```

One lookup instead of two or three.

**Before — manual Vec dedup:**
```rust
let mut seen = HashSet::new();
let mut unique = Vec::new();
for item in items {
    if seen.insert(item.id) {
        unique.push(item);
    }
}
```

This is fine if you need to preserve order and dedup by a key. But if you just need unique values:

**After (if order doesn't matter):**
```rust
let unique: HashSet<_> = items.into_iter().collect();
```

**After (if order matters and items are sortable):**
```rust
items.sort();
items.dedup();
```

**Iterating with index — prefer `enumerate`:**
```rust
// Before
for i in 0..items.len() {
    println!("{}: {}", i, items[i]);
}

// After
for (i, item) in items.iter().enumerate() {
    println!("{i}: {item}");
}
```

No bounds checking, no indexing — cleaner and faster.

---

## 13. Error Handling Design

**Use `thiserror` for library errors, `anyhow` for application errors.**

**Before — stringly-typed errors:**
```rust
fn parse(input: &str) -> Result<Data, String> {
    let val: i64 = input.parse().map_err(|e| format!("parse failed: {e}"))?;
    Ok(Data(val))
}
```

**After — typed errors with thiserror:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to parse integer")]
    InvalidInt(#[from] std::num::ParseIntError),
}

fn parse(input: &str) -> Result<Data, ParseError> {
    let val: i64 = input.parse()?;
    Ok(Data(val))
}
```

The `#[from]` attribute implements `From` automatically, so `?` just works.

**In application code where you don't need error variants:**
```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path)
        .context("failed to read config file")?;
    let config: Config = serde_json::from_str(&data)
        .context("failed to parse config")?;
    Ok(config)
}
```

`.context()` adds human-readable messages without defining error enums you'll never match on.

---

## 14. Ownership and Borrowing Ergonomics

**Before — unnecessary clone:**
```rust
fn process(items: &[String]) {
    for item in items {
        let owned = item.clone();
        send(owned);
    }
}
```

**After — take ownership if you need it:**
```rust
fn process(items: Vec<String>) {
    for item in items {
        send(item);
    }
}
```

If the function consumes the data, take it by value. The caller can `.clone()` if they still need it — that makes the allocation explicit at the call site.

**Before — borrowing what you'll immediately own:**
```rust
fn build_greeting(name: &str) -> String {
    let mut s = name.to_string();
    s.push_str("!");
    s
}
```

**After:**
```rust
fn build_greeting(name: &str) -> String {
    format!("{name}!")
}
```

**`Cow` for conditional ownership:**

When a function *usually* returns borrowed data but *sometimes* needs to allocate:

```rust
use std::borrow::Cow;

fn trim_prefix<'a>(s: &'a str, prefix: &str) -> Cow<'a, str> {
    match s.strip_prefix(prefix) {
        Some(rest) => Cow::Borrowed(rest),
        None => Cow::Borrowed(s),
    }
}
```

Better than always returning `String` when allocation is avoidable.

---

## 15. Trait Design for Usability

**Accept the most general input your function can handle:**

```rust
// Rigid — only takes &str
fn search(haystack: &str, needle: &str) -> bool

// Flexible — takes anything string-like
fn search(haystack: &str, needle: impl AsRef<str>) -> bool
```

But don't over-abstract. If you only ever call this with `&str`, keep it simple.

**Return concrete types unless you need abstraction:**

```rust
// Good — caller knows exactly what they get
fn get_names(&self) -> Vec<String>

// Also good — zero-allocation, lazy evaluation
fn get_names(&self) -> impl Iterator<Item = &str>
```

**Implement standard traits to be a good citizen:**
- `Display` — for user-facing output
- `Debug` — derive it on everything, no exceptions
- `Clone` — unless cloning would be semantically wrong
- `From`/`Into` — for natural type conversions
- `Default` — when there's an obvious "empty" state

---

## 16. Control Flow Simplification

**Before — boolean return from if/else:**
```rust
fn is_valid(x: i32) -> bool {
    if x > 0 && x < 100 {
        true
    } else {
        false
    }
}
```

**After:**
```rust
fn is_valid(x: i32) -> bool {
    x > 0 && x < 100
}
```

**Before — match on boolean:**
```rust
match condition {
    true => do_something(),
    false => do_other(),
}
```

**After:**
```rust
if condition {
    do_something()
} else {
    do_other()
}
```

**Before — negated condition with empty if-block:**
```rust
if !items.is_empty() {
    // do nothing
} else {
    return Err(Error::NoItems);
}
```

**After:**
```rust
if items.is_empty() {
    return Err(Error::NoItems);
}
```

**Before — if-let with only the else branch used:**
```rust
if let Some(_) = value {
    // nothing
} else {
    return Err(Error::Missing);
}
```

**After:**
```rust
if value.is_none() {
    return Err(Error::Missing);
}
```

Or if you need the binding:
```rust
let Some(v) = value else {
    return Err(Error::Missing);
};
```
