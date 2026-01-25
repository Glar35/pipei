# pipe{i}

`pipei` provides a zero-cost, type-safe way to chain multi-argument functions using method syntax. It turns a function call `f(x, y, z)` into a method call `x.pipe(f)(y, z)`. It also includes a `tap` operator for side-effects (logging, mutation) that returns the original value.

This project is inspired by the [UMCS proposal](https://internals.rust-lang.org/t/weird-syntax-idea-s-for-umcs/19200/35).
It generalizes the [`tap`](https://crates.io/crates/tap) crate to support multi-argument pipelines and fallible operations `?` without nesting closures.

**Note:** Requires `#![feature(impl_trait_in_assoc_type)]` on nightly.


To keep compile times fast, enable only the arities you need. 
The crate supports arities from 0 (a single argument) up to 50. Use features like `up_to_N` (where `N` is a multiple of 5) or specific individual arity features
```toml
[dependencies]
pipei = "*" # default: features = ["up_to_5"]
# pipei = { version = "*", features = ["up_to_20"] }  
# pipei = { version = "*", features = ["0", "1", "3", "4"] }```
```

## Basic chaining

`pipe` passes the value into the function and returns the result. `tap` inspects or mutates the input, ignores the result, and returns the original value.

**Unified Tap API**: `tap` methods seamlessly accept functions taking either `&Self` (immutable) or `&mut Self` (mutable).

```rust
use pipei::{Pipe, Tap};

fn add(x: i32, y: i32) -> i32 { x + y }
fn mul(x: i32, y: i32) -> i32 { x * y }
fn lin(x: i32, a: i32, b: i32) -> i32 { a * x + b }

let maybe_num = 2
    .pipe(add)(3)      
    .pipe(mul)(10)    
    .pipe(lin)(7, 1)
    .pipe(Option::Some)();

assert_eq!(maybe_num, Some(351));

fn log_val(x: &i32) { println!("val: {}", x); }
fn add_assign(x: &mut i32, y: i32) { *x += y; }

let val = 2
    .tap(log_val)()         // Immutable: passes &i32
    .tap(add_assign)(3)     // Mutable: passes &mut i32
    .tap(log_val)();

assert_eq!(val, 5);
```

## `Pipe` for method binding

`pipe` can be used to bound methods by partially applying an object to a method.

```rust
use pipei::Pipe;

struct Scalar(i32);
impl Scalar {
    fn linearize(&self, a: i32, b: i32) -> i32 { a * self.0 + b }
}

let scalar = Scalar(10);

// Extracting the bound method `scalar.linearize` as a standalone function.
let method_as_function = scalar.pipe(Scalar::linearize);

assert_eq!(method_as_function(1, 5), 15);
```

## `TapWith`

While `tap` works great for direct access, `tap_with` separates the projection logic from the side-effect logic. This is necessary when the adaptation is non-trivial (e.g., calling a method instead of simple dereferencing) or when inspecting specific fields.

```rust
use pipei::TapWith;

fn check_bytes(b: &[u8]) { assert_eq!(b[0], b'h'); }

let s = String::from("hello");
// "as_bytes" is a method, not a Deref, so automatic coercion won't work.
s.tap_with(|s| s.as_bytes(), check_bytes)();

struct Config { port: u16, host: String }
fn check_port(p: &u16) { assert!(*p > 1024, "Reserved port!"); }

let cfg = Config { port: 8080, host: "localhost".into() };
// Projects &Config -> &u16 to reuse a standard check function
cfg.tap_with(|c| &c.port, check_port)();
```

## `PipeRef`

`pipe_ref` allows extracting a sub-value (borrow) from a mutable parent for transformation chains without moving the parent.

```rust
use pipei::{PipeRef, Tap};

fn get_mut(v: &mut [i32; 3], i: usize) -> &mut i32 { &mut v[i] }

let mut data = [10, 20, 30];

// Start a pipe from &mut data, get a mutable reference to index 0
*data.pipe_ref(get_mut)(0) = 99;

assert_eq!(data[0], 99);
```

## Comparison with the `tap` crate

The [tap](https://crates.io/crates/tap) crate is the standard solution for continuous chaining. `pipei` extends this concept to multi-argument functions to address specific issues related to control flow, error handling, and nesting.

When function arguments are the results of other chains (nesting) and those chains involve fallible operations (using `?`), standard closure-based chaining becomes difficult to manage.

Consider a workflow where we load a background, load and resize an overlay, composite them, and save the result. Both `load` and `save` are fallible (return `Result`).

**Standard Rust:**
The logic reads "inside-out": `save` is written first, but executes last.
```rust
save(
    composite_onto(
        load("background.png")?,            
        resize(load("overlay.png")?, 50),   
        0.8                                 
    ),
    "result.png"                            
);
```

**Using `tap`:**
We can try to linearize it, but the secondary chain (`overlay`) must happen inside a closure. Because `load("overlay")?` uses the `?` operator, the closure itself returns a `Result`.
```rust
load("background.png")?
    .pipe(|bg| {
        // The `?` here forces this closure to return Result<Image, Error>
        let overlay = load("overlay.png")?
            .pipe(|fg| resize(fg, 50));
        
        // We must wrap the result in Ok() to satisfy the Result signature
        Ok(composite_onto(bg, overlay, 0.8))
    })? 
    .pipe(|img| save(img, "result.png"));
```

**Using `pipei`:**
The primary flow (`load` -> `composite` -> `save`) remains linear. The secondary flow (`overlay` -> `resize`) is handled inline. The `?` operator works naturally without changing the pipeline types or requiring `and_then`.
```rust
load("background.png")?
    .pipe(composite_onto)(
        load("overlay.png")?.pipe(resize)(50), 
        0.8,
    )
    .pipe(save)("result.png");
```