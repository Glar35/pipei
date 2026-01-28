# pipe{i}

`pipei` provides a zero-cost, type-safe way to chain multi-argument functions using method syntax. It turns a function call `f(x, y, z)` into a method call `x.pipe(f)(y, z)`. 
It also includes a `tap` operator for side-effects (logging, mutation) that returns the original value.

This project is inspired by the [UMCS proposal](https://internals.rust-lang.org/t/weird-syntax-idea-s-for-umcs/19200/35).
It generalizes the [`tap`](https://crates.io/crates/tap) crate to support multi-argument pipelines.

**Note:** Requires `#![feature(impl_trait_in_assoc_type)]` on nightly.


To keep compile times as fast as possible, enable only the arities you need. 
The crate supports arities from 0 (a single argument) up to 50. Use features like `up_to_N` (where `N` is a multiple of 5) or specific individual arity features
```toml
[dependencies]
pipei = "*" # default: features = ["up_to_5"]
# pipei = { version = "*", features = ["up_to_20"] }  
# pipei = { version = "*", features = ["0", "1", "3", "4"] }
```

## Basic chaining

`pipe` passes the value into the function and returns the result. `tap` inspects or mutates the input, ignores the result, and returns the original value.

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

assert_eq!(val, 5);
```

## `Pipe` for Method Binding

'pipe' can be used to convert a method into a standalone function. 
By binding a specific object as the 'self' argument, you create a reusable function that implicitly uses that object's state.
```rust
use pipei::Pipe;

struct Discount {
    rate: f64,
}

impl Discount {
    fn apply(&self, price: f64, quantity: i32) -> f64 {
        price * (quantity as f64) * (1.0 - self.rate)
    }
}

let season_pass = Discount { rate: 0.20 };

let calculate_total = season_pass.pipe(Discount::apply);

assert_eq!(calculate_total(100.0, 2), 160.0);
```

## `TapWith`

tap_with runs a side-effect on a projection of the value. 
This is useful for adapting types (e.g. calling a method to get a different view) or selecting specific fields to reuse generic validation logic.

```rust
use pipei::TapWith;

// 1. Adapting types (String -> &[u8])
fn validate_header(bytes: &[u8]) {
    assert_eq!(bytes[0], b'H');
}

let data = String::from("Header-Data");
data.tap_with(|s| s.as_bytes(), validate_header)();


// 2. Selecting fields
struct Server {
    port: u16,
    active: bool,
}

fn check_safe_port(port: &u16) {
    assert!(*port > 1024);
}

let srv = Server { port: 8080, active: true };
srv.tap_with(|s| &s.port, check_safe_port)();
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