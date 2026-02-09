# pipe{i}

_pipei_ allows writing `x.pipe(f)(y, z)` in place of `f(x, y, z)`, enabling method-style chaining and reusable partial application by returning a closure over the remaining arguments.
The library similarly provides a multi-argument `tap` operator for side effects that returns the original value.

This project is inspired by the [UMCS proposal](https://internals.rust-lang.org/t/weird-syntax-idea-s-for-umcs/19200/35). It requires nightly Rust for `#![feature(impl_trait_in_assoc_type)]`.

## Installation

To optimize compile time, enable only the arities you need (from 0 up to 50).
Use `up_to_N` features (available in multiples of five) or enable individual arity features.

```toml
[dependencies]
pipei = "*" # default: features = ["up_to_5"]
# pipei = { version = "*", features = ["up_to_20", "31"] }  
# pipei = { version = "*", features = ["0", "1", "3", "4"] }
```

## Basic chaining

`pipe` passes the value to the function and returns the result. 
`tap` passes the value for a side effect—logging, assertions, or mutation—and returns the original value.


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
    .tap(add_assign)(3);    // Mutable: passes &mut i32

assert_eq!(val, 5);
```

## Partial Application

`pipe` curries the first argument of a function, producing a standalone reusable function that accepts the remaining arguments.

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

`tap_with` takes a projection that returns an `Option`; if the result is `Some`, the side effect runs on the projected value.
This bridges the gap when a side effect’s signature does not match the receiver: the projection adapts one to the other, whether by accessing a field, calling `.as_ref()`, `.as_bytes()`, or any other transformation.
It subsumes the specialized methods from the [`tap`](https://crates.io/crates/tap) crate (`tap_ok`, `tap_dbg`, etc.) using a single generic projection.

```rust
use pipei::TapWith;

#[derive(Debug)]
struct Request { url: String, attempts: u32 }

fn track_retry(count: &mut u32) { *count += 1 }
fn log_status(code: &u32, count: u32) { /* ... */ }
fn log_trace(req: &Request, label: &str) { /* ... */ }

let mut req = Request { url: "https://api.rs".into(), attempts: 3 };

// Simulating tap's `tap_mut` on a field
(&mut req).tap_with(|r| Some(&mut r.attempts), track_retry)();

// Simulating tap's `tap_err` (only tap on error)
let res = Err::<Request, _>(503)
    .tap_with(|x| x.as_ref().err(), log_status)(req.attempts);

assert_eq!(res.unwrap_err(), 503);


// Simulating tap's `tap_dbg` (only tap in debug mode)
let final_req = req.tap_with(|r| {
    #[cfg(debug_assertions)] { Some(r) }
    #[cfg(not(debug_assertions))] { None }
    }, log_trace)("FINAL_STATE");


assert_eq!(final_req.attempts, 4);
```


## Comparison with the _tap_ crate

_pipei_ generalizes _tap_ to support multi-argument functions, reducing syntactic noise and simplifying control flow in pipelines involving `Result` or `Option`.

**Standard Rust:**
The reading order is inverted ("inside-out"): `save` is written first, but executes last.
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

**Using _tap_:**
Since `?` applies to the closure, the closure itself returns a `Result`. 
This forces manual `Ok` wrapping and an extra `?` after the `pipe` call.
```rust
load("background.png")?
    .pipe(|bg| {
        let overlay = load("overlay.png")?
            .pipe(|fg| resize(fg, 50));
        
        Ok(composite_onto(bg, overlay, 0.8))
    })? 
    .pipe(|img| save(img, "result.png"));
```

**Using _pipei_:**
The flow remains flat and `?` works naturally.
```rust
load("background.png")?
    .pipe(composite_onto)(
        load("overlay.png")?.pipe(resize)(50), 
        0.8,
    )
    .pipe(save)("result.png");
```