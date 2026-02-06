# pipe{i}

`pipei` provides a zero-cost, type-safe way to chain multi-argument functions using method syntax. It turns a function call `f(x, y, z)` into a method call `x.pipe(f)(y, z)`. 
It also includes a `tap` operator for side-effects (logging, mutation) that returns the original value.

This project is inspired by the [UMCS proposal](https://internals.rust-lang.org/t/weird-syntax-idea-s-for-umcs/19200/35).
It generalizes the [`tap`](https://crates.io/crates/tap) crate to support multi-argument pipelines.

**Note:** Requires `#![feature(impl_trait_in_assoc_type)]` on nightly.


To optimize compile times, enable only the arities you need (from 0 up to 50).
Use features like `up_to_N` (where `N` is a multiple of 5) or specific individual arity features
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

## Partial Application

`pipe` can pre-fill the first argument of a function, creating a standalone, reusable function that accepts the remaining arguments.

```rust
use pipei::Pipe;

struct Discount { rate: f64 }

impl Discount {
    fn apply(&self, price: f64) -> f64 {
        price * (1.0 - self.rate)
    }
}

let season_pass = Discount { rate: 0.20 };

// Equivalent to the (hypothetical): let apply_discount = season_pass.apply;
let apply_discount = season_pass.pipe(Discount::apply);

let prices = [100.0, 200.0, 300.0];
let discounted = prices.map(apply_discount);

assert_eq!(discounted, [80.0, 160.0, 240.0]);
```

## `TapWith`

Runs a side-effect on a projection of the value, then returns the original value. 
The side-effect only executes if the projection returns `Some`. 
This allows `tap_with` to handle field inspection, conditional flows (tap_ok, tap_some), and debug-only operations.

```rust
use pipei::TapWith;

struct Request { url: String, attempts: u32 }

fn log_audit(url: &str, id: u32) { /* */ }
fn log_retry(err: &str, count: u32) {  /* */ }
fn log_trace(req: &Request, label: &str) {  /* */  }

let req = Request { url: "https://api.rs".into(), attempts: 3 };

// tap on a (projection of a) field
(&req).tap_with(|r| Some(r.url.as_str()), log_audit)(101);

// Simulating tap_err (only tap an error)
let res: Result<Request, &str> = Err("Timeout")
    .tap_with(|x| x.err(), log_retry)(req.attempts);

// Simulating tap_dbg (only tap in debug mode)
let final_req = req.tap_with(|r| {
    #[cfg(debug_assertions)] { Some(r) }
    #[cfg(not(debug_assertions))] { None }
    }, log_trace)("FINAL_STATE");

assert_eq!(res.unwrap_err(), "Timeout");
assert_eq!(final_req.attempts, 3);
```


## Comparison with the `tap` crate

`pipei` generalizes `tap` to support multi-argument functions, reducing syntactic noise and simplifying control flow when pipelines involve `Result` or `Option` types.

**Standard Rust:**
The reading order is inverted ("inside-out"), as `save` is written first, but executes last.
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
Since `?` applies to the closure, the closure itself returns a `Result`. 
This forces manual `Ok` wrapping and an extra `?` after the pipe.
```rust
load("background.png")?
    .pipe(|bg| {
        let overlay = load("overlay.png")?
            .pipe(|fg| resize(fg, 50));
        
        Ok(composite_onto(bg, overlay, 0.8))
    })? 
    .pipe(|img| save(img, "result.png"));
```

**Using `pipei`:**
The flow remains flat and `?` works naturally.
```rust
load("background.png")?
    .pipe(composite_onto)(
        load("overlay.png")?.pipe(resize)(50), 
        0.8,
    )
    .pipe(save)("result.png");
```