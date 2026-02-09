# pipe{i}

_pipei_ allows writing `x.pipe(f)(y, z)` in place of `f(x, y, z)`, enabling method-style chaining and partial application for multi-argument functions.
It also provides `tap` and `tap_with` for multi-argument side effects that return the original value.

This project is inspired by the [UMCS (Unified Method Call Syntax) proposal](https://internals.rust-lang.org/t/weird-syntax-idea-s-for-umcs/19200). It requires nightly Rust for `#![feature(impl_trait_in_assoc_type)]`.

### Basic Chaining

`pipe` passes the value as the first argument to a function and returns the result.
`tap` passes the value to a function for a side effect, then returns the original value.

```rust
use pipei::{Pipe, Tap};

fn add(x: i32, y: i32) -> i32 { x + y }

let result = 2
    .pipe(add)(3)
    .pipe(|x, a, b| a * x + b)(10, 1)
    .pipe(Option::Some)();

assert_eq!(result, Some(51));

fn log(x: &i32) { println!("val: {}", x); }
fn add_assign(x: &mut i32, y: i32) { *x += y; }

let val = 2
    .tap(log)()             // Immutable: inferred &i32
    .tap(add_assign)(3);    // Mutable: inferred &mut i32

assert_eq!(val, 5);
```

### Partial Application

Because `pipe` returns a closure over the remaining arguments, it doubles as partial application.
```rust
use pipei::Pipe;

struct Discount { rate: f64 }

impl Discount {
    fn apply(&self, price: f64) -> f64 {
        price * (1.0 - self.rate)
    }
}

let season_pass = Discount { rate: 0.20 };

// Equivalent to (the hypothetical): let apply_discount = season_pass.apply;
let apply_discount = season_pass.pipe(Discount::apply);

let prices = [100.0, 200.0, 300.0];
let discounted = prices.map(apply_discount);

assert_eq!(discounted, [80.0, 160.0, 240.0]);
```

### Projection

`tap_with` lets you compose an existing function with a projection on the receiver when the function's signature doesn't match the receiver directly. 
The projection returns an `Option` to allow for _conditional execution_; returning `None` skips the side effect. 
Like `tap`, the original value is always returned.

```rust
use pipei::TapWith;
struct Request { url: String, attempts: u32 }

fn track_retry(count: &mut u32) { *count += 1 }
fn log_trace<T: core::fmt::Debug>(val: &T, label: &str) { /* ... */ }

let mut req = Request { url: "https://pipei.rs".into(), attempts: 3 };

// Compose `track_retry` with the projection to the `attempts` field
(&mut req).tap_with(|r| Some(&mut r.attempts), track_retry)();
assert_eq!(req.attempts, 4);

// tap only on Err
let res = Err::<(), _>(503)
    .tap_with(|x| x.as_ref().err(), log_trace)("request failed");
assert_eq!(res.unwrap_err(), 503);

// tap only in debug builds
let req = req.tap_with(|r| {
    #[cfg(debug_assertions)] { Some(r) }
    #[cfg(not(debug_assertions))] { None }
    }, log_trace)("FINAL");
assert_eq!(req.attempts, 4);
```

### Error Handling

The [_tap_](https://crates.io/crates/tap) crate provides single-argument `pipe` and `tap` traits.
_pipei_ generalizes these to multi-argument functions, so arguments are passed directly rather than nested in closures.
This has the advantage of simplifying fallible pipelines, particularly when using control flow operations.

In the following example, the reading order is inverted ("inside-out"): `save` is written first, but executes last.
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

**With the _tap_ crate:**
Since `?` applies inside the closure, the closure returns a `Result`, forcing manual `Ok` wrapping and an extra `?`.
```rust
load("background.png")?
    .pipe(|bg| {
        let overlay = load("overlay.png")?
            .pipe(|fg| resize(fg, 50));
        
        Ok(composite_onto(bg, overlay, 0.8))
    })? 
    .pipe(|img| save(img, "result.png"));
```

**With _pipei_:**
```rust
load("background.png")?
    .pipe(composite_onto)(
        load("overlay.png")?.pipe(resize)(50), 
        0.8,
    )
    .pipe(save)("result.png");
```

### Feature Flags

To optimize compile time, enable only the arities you need (from 0 up to 50).
Use `up_to_N` features (available in multiples of five) or enable individual arity features.

```toml
[dependencies]
pipei = "*" # default: features = ["up_to_10"]
# pipei = { version = "*", features = ["up_to_20", "31"] }  
# pipei = { version = "*", features = ["0", "1", "3", "4"] }
```