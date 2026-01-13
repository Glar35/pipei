# pipei

`pipei` provides `pipe{i}` and `tap{i}` traits that enable point-free (no closures) multi-argument function chaining syntax.
They bind the receiver as the first argument of `f` and return a closure for the remaining arguments.
* **Pipe**: Transforms the input. Returns the result of the function `f`.
* **Tap**: Inspects or mutates the input. Ignores the result of `f` and returns the original value.

The `_with` variants project the input (e.g., viewing `String` as `str` or `Vec<T>` as `[T]`) before passing it to the function.
The `_with_mut` variants allow side effects on a mutable reference, accepting both mutable functions (like `Vec::push`) and immutable functions (like `len`).

## Enabling arities

Enable the arities you need via features:

```toml
[dependencies]
pipei = "0.1" # default: features = ["up_to_5"]
# pipei = { version = "0.1", features = ["up_to_10"] }
# pipei = { version = "0.1", features = ["0", "1", "3", "4"] }
```

## Basic chaining (by value)

`pipe` passes the value into the function and returns the result. `tap` moves the value in, runs the function, and returns the original value.

**Unified Tap API**: `tap` methods seamlessly accept functions taking either `&Self` (immutable) or `&mut Self` (mutable).

```rust
use pipei::{Pipe1, Pipe2, Tap1, Tap2};

fn add(x: i32, y: i32) -> i32 { x + y }
fn mul(x: i32, y: i32) -> i32 { x * y }
fn lin(x: i32, a: i32, b: i32) -> i32 { a * x + b }

let out = 2
    .pipe1(add)(3)      // 2 + 3 = 5
    .pipe1(mul)(10)     // 5 * 10 = 50
    .pipe2(lin)(7, 1);  // 50 * 7 + 1 = 351

assert_eq!(out, 351);

fn log_val(x: &i32, label: &str) { println!("{}: {}", label, x); }
fn add_assign(x: &mut i32, y: i32) { *x += y; }

let val = 2
    .tap1(log_val)("init")     // Immutable: passes &i32
    .tap1(add_assign)(3)       // Mutable: passes &mut i32
    .tap1(log_val)("result");

assert_eq!(val, 5);
```

## Arity 0 (Pipe0 / Tap0)

`Pipe0` is useful for passing the receiver to a function that takes only one argument, or for wrapping the receiver in a constructor (like `Some` or `Ok`) without extra parentheses.

```rust
use pipei::{Pipe0, Tap0};

fn get_len(s: String) -> usize { s.len() }
fn log_val(s: &String) { println!("val: {}", s); }
fn clear_str(s: &mut String) { s.clear(); }

let maybe_num = "hello".to_string()
    .pipe0(get_len)()
    .pipe0(Option::Some)(); // No need for wrapper syntax

assert_eq!(maybe_num, Some(5));

// Works with both immutable and mutable functions
let s = "hello".to_string()
    .tap0(log_val)()    // Inspect
    .tap0(clear_str)(); // Mutate

assert_eq!(s, "");
```

## Borrowed views (Projection)

Use `_with` variants to apply a projection (like `Borrow::borrow` or `AsRef::as_ref`) before calling the function. This is useful for type adaptation or component inspection.

```rust
use pipei::{Tap0Ref};
use std::path::{Path, PathBuf};

fn log_ext(p: &Path) { 
    println!("File type: {:?}", p.extension().unwrap_or_default()); 
}

struct Config { port: u16, host: String }
fn check_port(p: &u16) { assert!(*p > 1024, "Reserved port!"); }

// 1. Type Adaptation: Project PathBuf -> &Path
let path = PathBuf::from("data.json");
path.tap0_with(|x| x.as_ref(), log_ext)(); 

// 2. Component Inspection: Validate a field
let cfg = Config { port: 8080, host: "127.0.0.1".into() };
let ready_cfg = cfg.tap0_with(|c| &c.port, check_port)();
```

## Mutable views

`tap{i}_with_mut` allows chaining side effects on a mutable reference. It accepts both mutable and immutable functions.

```rust
use pipei::{Pipe1Ref, Tap1Ref};

fn log_vec(v: &Vec<i32>) { println!("len: {}", v.len()); }
fn push_ret(v: &mut Vec<i32>, x: i32) -> &mut Vec<i32> { v.push(x); v }

let mut v1 = vec![];
v1.pipe1_with_mut(|x| x, push_ret)(1);
assert_eq!(v1, vec![1]);

let mut v2 = vec![];
v2.tap0_with_mut(|x| x, log_vec)()          // Immutable Fn(&Vec) works on mutable view
  .tap1_with_mut(|x| x, Vec::push)(1);      // Mutable Fn(&mut Vec) works

assert_eq!(v2, vec![1]);
```

## Comparison with the `tap` crate

The [tap](https://crates.io/crates/tap) crate is the standard solution for continuous chaining.
`pipei` extends this concept to multi-argument functions to address issues related to control flow and nesting depth.

### 1. Control Flow (Using The `?` Operator)

When dealing with multi-argument functions, because `tap`'s methods must take closure, 
`?` (and `return`) inside it apply to the closure rather than the surrounding function, 
which may force you to carry `Result` through the chain.

In the following example, we are forced to break the method chaining and to use intermediate variables.

**Standard Rust:**
```rust
fn render_value(raw: &str) -> Result<String, ()> {
    let n: i32 = raw.trim().parse().map_err(|_| ())?;
    let q = checked_div(n, 2)?;
    Ok(surround(q, "=", "=").to_ascii_uppercase())
}
```

**Using `tap`:**
Because `tap`'s `pipe` takes a closure, using `?` inside that closure makes the closure return a `Result`, 
so you end up carrying `Result` through the chain (e.g. via `map`/`and_then`) instead of writing `?` at each step.
```rust
fn render_value(raw: &str) -> Result<String, ()> {
    raw.trim()
        .parse::<i32>().map_err(|_| ())
        .pipe(|r| r.and_then(|x| checked_div(x, 2)))
        .pipe(|r| r.map(|x| surround(x, "=", "=")))
        .pipe(|r| r.map(|s| s.to_ascii_uppercase()))
}
```

**Using `pipei`:**
With `pipei`, arguments are evaluated before the call, so the `?` operator works exactly as intended.
```rust
fn render_value(raw: &str) -> Result<String, ()> {
    raw.trim()
        .parse::<i32>().map_err(|_| ())?
        .pipe1(checked_div)(2)?
        .pipe2(surround)("=", "=")
        .to_ascii_uppercase()
        .pipe0(Ok)()
}
```

### 2. Recursive Nesting

When function arguments are results of other chains, standard chaining forces deep closure nesting. 
`pipei` maintains a flat structure. To illustrate this, consider the following example:

**Standard Rust:**
The logic is "inside-out": `save` is written first, but happens last.
```rust
save(
    composite_onto(
        load("background.png")?,            // 1. Load Background
        resize(load("overlay.png")?, 50),   // 2. Load & Resize Overlay
        0.8                                 // 3. Overlay opacity
    ),
    "result.png"                            // 4. Save
);
```

**Using `tap`:** We restore the top-to-bottom flow, but processing the second argument (the overlay) requires opening a nested closure, adding visual clutter and further complicating control flow issues.
```rust
load("background.png")?
    .pipe(|bg| {
        load("overlay.png")
            .pipe(|r| r.map(|fg| resize(fg, 50)))
            .map(|fg| composite_onto(bg, fg, 0.8))
    })
    .and_then(|img| save(img, "result.png"));
```

**Using `pipei`:**
The primary flow (`load` -> `composite_onto` -> `save`) remains linear. The secondary flow (`overlay` -> `resize`) is handled inline without closures.
```rust
load("background.png")?
    .pipe2(composite_onto)(
        load("overlay.png")?.pipe1(resize)(50),
        0.8,
    )
    .pipe1(save)("result.png")
```