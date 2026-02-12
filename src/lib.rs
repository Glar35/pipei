#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]

//! # pipei
//!
//! A zero-cost library for chaining multi-argument function calls in method syntax.
//!
//! `pipe` allows writing `x.pipe(f)(y, z)` instead of `f(x, y, z)` by currying the receiver into the first argument.
//! `tap` provides the same call form for side effects: it passes the value to a function for inspection or mutation and then returns the original value.
//!
//! ## Extension traits
//!
//! * **[`Pipe::pipe`]:** Curries `self` into the first argument of a function, returning the result.
//! * **[`Tap::tap`]:** Passes `self` to a function for inspection or mutation, then returns the original (now possibly modified) value.
//! * **[`TapWith::tap_proj`]:** Like `tap`, but first applies a projection to extract a sub-reference.
//! * **[`TapWith::tap_cond`]:** Like `tap_proj`, but the projection returns `Option`; the side effect only runs on `Some`.
//!
//! ```rust
//! # use pipei::{Pipe, Tap};
//! fn add(a: i32, b: i32) -> i32 { a + b }
//! fn log(x: &i32) { println!("{x}"); }
//!
//! let result = 1
//!     .pipe(add)(2)
//!     .tap(log)()
//!     .pipe(Option::Some)();
//!
//! assert_eq!(result, Some(3));
//! ```

// ============================================================================================
// Internal mechanism
// ============================================================================================

#[doc(hidden)]
/// Marker type: pass the pipeline value by shared reference (`&T`).
pub struct Imm;
#[doc(hidden)]
/// Marker type: pass the pipeline value by exclusive reference (`&mut T`).
pub struct Mut;
#[doc(hidden)]
/// Marker type: pass the pipeline value by value (`T`).
pub struct Own;
#[doc(hidden)]
/// Marker type: `tap` semantics (return the original value).
pub struct TapMark;
#[doc(hidden)]
/// Marker type: `pipe` semantics (return the function's result).
pub struct PipeMark;
#[doc(hidden)]
/// Marker type: `tap_proj` semantics (unconditional projection).
pub struct Proj;
#[doc(hidden)]
/// Marker type: `tap_cond` semantics (conditional projection via Option).
pub struct Cond;

#[doc(hidden)]
/// Internal: curries a function's first argument, producing a closure over the remaining arguments.
pub trait Curry<const ARITY: usize, Args, AState, RState, MARK, A0: ?Sized, R: ?Sized> {
    type Curry;
    fn curry(self, arg0: A0) -> Self::Curry;
}

#[doc(hidden)]
/// Internal: curries a function's first argument through a projection (conditional or unconditional).
pub trait CurryWith<const ARITY: usize, Args, State, MARK, A0: ?Sized, P, R: ?Sized> {
    type Curry;
    fn curry_with(self, arg0: A0, proj: P) -> Self::Curry;
}

// ============================================================================================
// Public Extension Traits
// ============================================================================================

/// Extension trait for transforming values.
pub trait Pipe<const ARITY: usize, AState, RState> {
    /// Curries `self` as the first argument of `f`, returning a closure over
    /// the remaining arguments. The returned closure is a standalone value,
    /// so `pipe` doubles as partial application.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pipei::Pipe;
    /// fn add(x: i32, y: i32) -> i32 { x + y }
    ///
    /// let result = 2
    ///     .pipe(add)(3)
    ///     .pipe(|x, a, b| a * x + b)(10, 1)
    ///     .pipe(Option::Some)();
    ///
    /// assert_eq!(result, Some(51));
    ///
    /// struct Threshold(i32);
    /// impl Threshold {
    ///     fn check(&self, val: i32) -> bool { val > self.0 }
    /// }
    ///
    /// let is_high = Threshold(50).pipe(Threshold::check);
    /// assert_eq!([20, 60, 80].map(is_high), [false, true, true]);
    ///
    /// fn first_mut(slice: &mut [i32; 3]) -> &mut i32 { &mut slice[0] }
    ///
    /// let mut data = [10, 20, 30];
    /// *(&mut data).pipe(first_mut)() = 99;
    /// assert_eq!(data[0], 99);
    /// ```
    #[inline(always)]
    fn pipe<R, F, Args>(self, f: F) -> F::Curry
    where
        F: Curry<ARITY, Args, AState, RState, PipeMark, Self, R>,
        Self: Sized,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, AState, RState, T> Pipe<ARITY, AState, RState> for T {}

/// Extension trait for running side effects, returning the original value.
pub trait Tap<const ARITY: usize, State> {
    /// Passes `self` into `f` for inspection or mutation, then returns the
    /// original (possibly modified) value. The function receives `self` by
    /// shared or exclusive reference depending on its signature.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pipei::Tap;
    /// fn log(x: &i32) { println!("val: {x}"); }
    /// fn assert_between(x: &i32, lo: i32, hi: i32) { assert!(*x >= lo && *x <= hi); }
    /// fn add_assign(x: &mut i32, y: i32) { *x += y; }
    ///
    /// let result = 15
    ///     .tap(log)()
    ///     .tap(assert_between)(0, 100)
    ///     .tap(add_assign)(3);
    ///
    /// assert_eq!(result, 18);
    ///
    /// struct State { count: i32 }
    ///
    /// let s = State { count: 0 }
    ///     .tap(|s: &mut State| s.count += 1)()
    ///     .tap(|s: &mut State| s.count *= 10)();
    ///
    /// assert_eq!(s.count, 10);
    /// ```
    #[inline(always)]
    fn tap<R, F, Args>(self, f: F) -> F::Curry
    where
        F: Curry<ARITY, Args, State, Own, TapMark, Self, R>,
        Self: Sized,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, State, T> Tap<ARITY, State> for T {}

/// Extension trait for running side effects on a projection (conditional or unconditional) of the value.
pub trait TapWith<const ARITY: usize, State> {
    /// Applies a projection to `self`, then runs `f` on the projected reference.
    /// The projection returns `&T` or `&mut T` directly — it always runs.
    /// The original value is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pipei::TapWith;
    /// struct Pair { a: i32, b: i32 }
    /// fn check(v: &i32) { assert!(*v > 0); }
    /// fn increment(v: &mut i32) { *v += 1; }
    ///
    /// let p = Pair { a: 1, b: 2 }
    ///     .tap_proj(|p: &Pair| &p.a, check)()
    ///     .tap_proj(|p| &mut p.b, increment)();
    /// assert_eq!(p.b, 3);
    /// ```
    #[inline(always)]
    fn tap_proj<R, F, P, Args>(self, proj: P, f: F) -> F::Curry
    where
        F: CurryWith<ARITY, Args, State, Proj, Self, P, R>,
        Self: Sized,
    {
        f.curry_with(self, proj)
    }

    /// Runs a side effect on a projection of `self`. The projection returns
    /// an `Option`; if `Some`, the side effect runs on the projected value.
    /// If `None`, the side effect is skipped. In both cases, `self` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pipei::TapWith;
    /// #[derive(Debug)]
    /// struct Request { url: String, attempts: u32 }
    ///
    /// fn track_retry(count: &mut u32) { *count += 1 }
    /// fn log_status(code: &u32, url: &str, count: u32) { eprintln!("{url}: error {code} (attempt {count})"); }
    /// fn log_trace<T: core::fmt::Debug>(val: &T, label: &str) { eprintln!("{label}: {val:?}"); }
    ///
    /// let mut req = Request { url: "[https://pipei.rs](https://pipei.rs)".into(), attempts: 3 };
    ///
    /// // project to a mutable field
    /// (&mut req).tap_cond(|r| Some(&mut r.attempts), track_retry)();
    /// assert_eq!(req.attempts, 4);
    ///
    /// // tap only on Err
    /// let res = Err::<Request, _>(503)
    ///     .tap_cond(|x| x.as_ref().err(), log_status)(&req.url, req.attempts);
    /// assert_eq!(res.unwrap_err(), 503);
    ///
    /// // tap only in debug builds
    /// let req = req.tap_cond(|r| {
    ///     #[cfg(debug_assertions)] { Some(r) }
    ///     #[cfg(not(debug_assertions))] { None }
    /// }, log_trace)("FINAL");
    /// assert_eq!(req.attempts, 4);
    /// ```
    #[inline(always)]
    fn tap_cond<R, F, P, Args>(self, proj: P, f: F) -> F::Curry
    where
        F: CurryWith<ARITY, Args, State, Cond, Self, P, R>,
        Self: Sized,
    {
        f.curry_with(self, proj)
    }
}
impl<const ARITY: usize, State, T> TapWith<ARITY, State> for T {}

// ============================================================================================
// Macro Logic
// ============================================================================================

macro_rules! impl_arity {
    ($N:literal, $feat:literal, [ $($Args:ident),* ], $TupleType:ty) => {
        const _: () = {
            #[cfg(feature = $feat)]
            use crate::{Imm, Curry, CurryWith, Mut, Own, PipeMark, TapMark, Proj, Cond};

            // --- Pipe ---
            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> Curry<$N, $TupleType, Imm, Own, PipeMark, A0, R> for F
            where F: for<'b> Fn(&'b A0, $($Args),*) -> R {
                type Curry = impl Fn($($Args),*) -> R;
                #[inline(always)] fn curry(self, arg0: A0) -> Self::Curry {
                    move |$($Args),*| self(&arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> Curry<$N, $TupleType, Own, Own, PipeMark, A0, R> for F
            where F: FnOnce(A0, $($Args),*) -> R {
                type Curry = impl FnOnce($($Args),*) -> R;
                #[inline(always)] fn curry(self, arg0: A0) -> Self::Curry {
                    move |$($Args),*| self(arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> Curry<$N, $TupleType, Mut, Own, PipeMark, A0, R> for F
            where F: for<'b> FnMut(&'b mut A0, $($Args),*) -> R {
                type Curry = impl FnMut($($Args),*) -> R;
                #[inline(always)] fn curry(mut self, mut arg0: A0) -> Self::Curry {
                    move |$($Args),*| self(&mut arg0, $($Args),*)
                }
            }

            // --- Tap ---
            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> Curry<$N, $TupleType, Imm, Own, TapMark, A0, R> for F
            where F: FnOnce(& A0, $($Args),*) -> R {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry(self, arg0: A0) -> Self::Curry {
                    move |$($Args),*| { self(&arg0, $($Args),*); arg0 }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> Curry<$N, $TupleType, Mut, Own, TapMark, A0, R> for F
            where F: FnOnce(&mut A0, $($Args),*) -> R {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry(self, mut arg0: A0) -> Self::Curry {
                    move |$($Args),*| { self(&mut arg0, $($Args),*); arg0 }
                }
            }

            // --- Tap Proj (CurryWith + Proj) ---
            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> CurryWith<$N, $TupleType, Imm, Proj, A0, P, R> for F
            where
                P: for<'b> FnOnce(&'b A0) -> &'b T,
                F: FnOnce(&T, $($Args),*) -> R
            {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry_with(self, arg0: A0, proj: P) -> Self::Curry {
                    move |$($Args),*| {
                        self(proj(&arg0), $($Args),*);
                        arg0
                    }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> CurryWith<$N, $TupleType, Mut, Proj, A0, P, R> for F
            where
                P: for<'b> FnOnce(&'b mut A0) -> &'b mut T,
                F: FnOnce(&mut T, $($Args),*) -> R
            {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry_with(self, mut arg0: A0, proj: P) -> Self::Curry {
                    move |$($Args),*| {
                        self(proj(&mut arg0), $($Args),*);
                        arg0
                    }
                }
            }

            // --- Tap Cond (CurryWith + Cond) ---
            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> CurryWith<$N, $TupleType, Imm, Cond, A0, P, R> for F
            where
                P: for<'b> FnOnce(&'b A0) -> Option<&'b T>,
                F: FnOnce(&T, $($Args),*) -> R
            {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry_with(self, arg0: A0, proj: P) -> Self::Curry {
                    move |$($Args),*| {
                        if let Some(v) = proj(&arg0) { self(v, $($Args),*); }
                        arg0
                    }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> CurryWith<$N, $TupleType, Mut, Cond, A0, P, R> for F
            where
                P: for<'b> FnOnce(&'b mut A0) -> Option<&'b mut T>,
                F: FnOnce(& mut T, $($Args),*) -> R
            {
                type Curry = impl FnOnce($($Args),*) -> A0;
                #[inline(always)] fn curry_with(self, mut arg0: A0, proj: P) -> Self::Curry {
                    move |$($Args),*| {
                        if let Some(v) = proj(&mut arg0) { self(v, $($Args),*); }
                        arg0
                    }
                }
            }
        };
    };
}

macro_rules! generate_pipeline {
    ( (0, $feat0:literal), $($rest:tt)* ) => {
        impl_arity!(0, $feat0, [], ());
        generate_pipeline!(@recurse [] ; $($rest)* );
    };

    (@recurse $acc:tt ; ) => {};

    (@recurse [ $($Acc:ident),* ] ; ($N:literal, $feat:literal, $Next:ident) $(, ($Ns:literal, $feats:literal, $Nexts:ident))* $(,)? ) => {
        impl_arity!($N, $feat, [ $($Acc,)* $Next ], ( $($Acc,)* $Next, ) );
        generate_pipeline!(@recurse [ $($Acc,)* $Next ] ; $( ($Ns, $feats, $Nexts) ),* );
    };
}

generate_pipeline! {
    (0, "0"),
    (1, "1", P1), (2, "2", P2), (3, "3", P3), (4, "4", P4), (5, "5", P5),
    (6, "6", P6), (7, "7", P7), (8, "8", P8), (9, "9", P9), (10, "10", P10),
    (11, "11", P11), (12, "12", P12), (13, "13", P13), (14, "14", P14), (15, "15", P15),
    (16, "16", P16), (17, "17", P17), (18, "18", P18), (19, "19", P19), (20, "20", P20),
    (21, "21", P21), (22, "22", P22), (23, "23", P23), (24, "24", P24), (25, "25", P25),
    (26, "26", P26), (27, "27", P27), (28, "28", P28), (29, "29", P29), (30, "30", P30),
    (31, "31", P31), (32, "32", P32), (33, "33", P33), (34, "34", P34), (35, "35", P35),
    (36, "36", P36), (37, "37", P37), (38, "38", P38), (39, "39", P39), (40, "40", P40),
    (41, "41", P41), (42, "42", P42), (43, "43", P43), (44, "44", P44), (45, "45", P45),
    (46, "46", P46), (47, "47", P47), (48, "48", P48), (49, "49", P49), (50, "50", P50),
}

// ============================================================================================
// Tests
// ============================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "0")]
    fn test_simple_pipe() {
        fn add_one(x: i32) -> i32 {
            x + 1
        }
        assert_eq!(1.pipe(add_one)(), 2);
    }

    #[test]
    #[cfg(feature = "1")]
    fn test_pipe_arity() {
        fn sub(x: i32, y: i32) -> i32 {
            x - y
        }
        assert_eq!(10.pipe(sub)(4), 6);
    }

    #[test]
    fn test_tap_cond_immutable() {
        struct Container {
            val: i32,
        }
        fn check_val(v: &i32) {
            assert_eq!(*v, 10);
        }

        let c = Container { val: 10 };
        // Explicit typing needed to resolve ambiguity between Imm and Mut source paths
        let res = c.tap_cond(|x: &Container| Some(&x.val), check_val)();
        assert_eq!(res.val, 10);
    }

    #[test]
    fn test_tap_proj_immutable() {
        struct Container {
            val: i32,
        }
        fn check_val(v: &i32) {
            assert_eq!(*v, 10);
        }

        let c = Container { val: 10 };
        let res = c.tap_proj(|x: &Container| &x.val, check_val)();
        assert_eq!(res.val, 10);
    }

    #[test]
    fn test_tap_cond_mutable() {
        struct Container {
            val: i32,
        }
        fn add_one(v: &mut i32) {
            *v += 1;
        }

        let c = Container { val: 10 };
        let res = c.tap_cond(|x| Some(&mut x.val), add_one)();
        assert_eq!(res.val, 11);
    }

    #[test]
    fn test_tap_proj_mutable() {
        struct Container {
            val: i32,
        }
        fn add_one(v: &mut i32) {
            *v += 1;
        }

        let c = Container { val: 10 };
        let res = c.tap_proj(|x| &mut x.val, add_one)();
        assert_eq!(res.val, 11);
    }

    #[test]
    fn test_pipe_mutable_borrow() {
        let mut data = [10, 20, 30];
        fn first_mut(slice: &mut [i32; 3]) -> &mut i32 {
            &mut slice[0]
        }

        let f: &mut i32 = (&mut data).pipe(first_mut)();
        *f = 99;
        assert_eq!(data[0], 99);
    }

    #[test]
    fn test_chaining_workflow() {
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        fn double(x: i32) -> i32 {
            x * 2
        }

        let res = 10.pipe(add)(5) // 15
            .pipe(double)() // 30
        .tap(|x: &i32| assert_eq!(*x, 30))();

        assert_eq!(res, 30);
    }

    #[test]
    fn test_mutable_tap_chain() {
        struct State {
            count: i32,
        }
        let s = State { count: 0 };

        let res = s.tap(|s: &mut State| s.count += 1)().tap(|s: &mut State| s.count += 2)();

        assert_eq!(res.count, 3);
    }

    #[test]
    fn bound_method_as_callback() {
        struct Button {
            id: usize,
        }
        impl Button {
            fn on_click(&self, prime: usize) -> usize {
                self.id % prime
            }
        }

        let buttons = [Button { id: 5 }, Button { id: 6 }];

        // 1. Make the array mutable and wrap items in Option
        let callbacks: [Option<_>; 2] =
            core::array::from_fn(|i| Some((&buttons[i]).pipe(Button::on_click)));

        for (cb, res) in callbacks.into_iter().zip([2, 0]) {
            let cb = cb.unwrap();
            assert_eq!(cb(3), res);
        }
    }

    #[test]
    fn unboxed_bound_methods() {
        struct Threshold(i32);
        impl Threshold {
            fn check(&self, val: i32) -> bool {
                val > self.0
            }
        }

        let low = Threshold(10);
        let high = Threshold(50);

        let mut validators = [
            Some(low.pipe(Threshold::check)),
            Some(high.pipe(Threshold::check)),
        ];

        assert!(validators[0].take().unwrap()(20));
        assert!(!validators[1].take().unwrap()(20));
    }

    #[test]
    fn server_check() {
        struct Server<'a> {
            ip: &'a str,
            port: u16,
        }

        // Reusable logic that checks raw bytes
        fn check_ipv4(bytes: &[u8]) {
            assert!(bytes.contains(&b'.'));
        }

        let s = Server {
            ip: "127.0.0.1",
            port: 8080,
        };

        // tap_proj: projection always succeeds
        (&s).tap_proj(|x| x.ip.as_bytes(), check_ipv4)();
        assert_eq!(s.port, 8080);

        let s = s.tap_proj(|x| x.ip.as_bytes(), check_ipv4)();
        assert_eq!(s.port, 8080);
    }

    #[test]
    fn tap_extended() {
        fn assert_lt(x: &i32, n: i32) {
            assert!(*x < n)
        }

        let val = 0.tap_cond(|x| if *x < 5 { Some(x) } else { None }, assert_lt)(5);
        assert_eq!(val, 0)
    }

    #[test]
    fn tap_proj_doc() {
        #[derive(Debug)]
        struct Request {
            _url: &'static str,
            attempts: u32,
        }

        fn track_retry(count: &mut u32) {
            *count += 1
        }

        let mut req = Request {
            _url: "https://api.rs",
            attempts: 3,
        };

        // tap_proj: always project to a mutable field
        (&mut req).tap_proj(|r| &mut r.attempts, track_retry)();
        assert_eq!(req.attempts, 4);
    }

    #[test]
    fn tap_cond_doc() {
        #[derive(Debug)]
        struct Request {
            _url: &'static str,
            attempts: u32,
        }

        fn track_retry(count: &mut u32) {
            *count += 1
        }
        fn log_status(_code: &u32, _count: u32) { /*   */
        }
        fn log_trace(_req: &Request, _label: &str) { /*   */
        }

        let mut req = Request {
            _url: "https://api.rs",
            attempts: 3,
        };

        // tap_mut a field
        (&mut req).tap_cond(|r| Some(&mut r.attempts), track_retry)();

        assert_eq!(req.attempts, 4);

        // tap_err (only tap on error)
        let res = Err::<Request, _>(503).tap_cond(|x| x.as_ref().err(), log_status)(req.attempts);

        assert_eq!(res.unwrap_err(), 503);

        // tap_dbg (only tap in debug mode)
        let req = req.tap_cond(
            |r| {
                #[cfg(debug_assertions)]
                {
                    Some(r)
                }
                #[cfg(not(debug_assertions))]
                {
                    None
                }
            },
            log_trace,
        )("FINAL_STATE");

        assert_eq!(req.attempts, 4);
    }

    #[test]
    fn tap_extended_mut() {
        fn take(x: &mut i32, n: i32) {
            *x -= n;
        }

        let val = 10.tap_cond(|x| if *x >= 5 { Some(x) } else { None }, take)(5);
        assert_eq!(val, 5)
    }

    #[test]
    fn check_reusability() {
        struct Discount {
            rate: f64,
        }

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
    }
}

#[cfg(test)]
mod extended_tap_tests {
    use super::*;

    fn log_val(_v: &i32) {}
    fn log_str(_s: &&str) {}
    fn mutate_val(v: &mut i32) {
        *v += 10;
    }

    #[test]
    fn test_simulate_tap_some() {
        let opt = Some(42);
        let res = opt.tap_cond(|x: &Option<i32>| x.as_ref(), log_val)();
        assert_eq!(res, Some(42));

        let none: Option<i32> = None;
        let res_none = none.tap_cond(|x: &Option<i32>| x.as_ref(), log_val)();
        assert_eq!(res_none, None);
    }

    #[test]
    fn test_simulate_tap_ok() {
        let res: Result<i32, &str> = Ok(100);
        let final_res = res.tap_cond(|x: &Result<i32, &str>| x.as_ref().ok(), log_val)();
        assert_eq!(final_res, Ok(100));
    }

    #[test]
    fn test_simulate_tap_err() {
        let res: Result<i32, &str> = Err("critical failure");
        let final_res = res.tap_cond(|x: &Result<i32, &str>| x.as_ref().err(), log_str)();
        assert_eq!(final_res, Err("critical failure"));
    }

    #[test]
    fn test_simulate_conditional_mutation() {
        let val = Some(5);
        let res = val.tap_cond(|x: &mut Option<i32>| x.as_mut(), mutate_val)();
        assert_eq!(res, Some(15));
    }

    #[test]
    fn test_simulate_tap_dbg() {
        fn my_dbg<T: core::fmt::Debug>(_v: &T) {}
        let value = 500;

        let res = value.tap_cond(
            |x: &i32| {
                #[cfg(debug_assertions)]
                {
                    Some(x)
                }
                #[cfg(not(debug_assertions))]
                {
                    None
                }
            },
            my_dbg,
        )();

        assert_eq!(res, 500);
    }
}

#[cfg(test)]
mod reference_tap_tests {
    use super::*;

    fn log_val(_v: &i32) {}
    fn log_str(_s: &str) {}
    fn mutate_val(v: &mut i32) {
        *v += 10;
    }

    #[test]
    fn test_ref_tap_some() {
        let opt = Some(42);
        let _ = (&opt).tap_cond(|x: &&Option<i32>| x.as_ref(), log_val)();
        assert_eq!(opt, Some(42));
    }

    #[test]
    fn test_ref_tap_ok() {
        let res: Result<i32, &str> = Ok(100);
        let _ = (&res).tap_cond(|x: &&Result<i32, &str>| x.as_ref().ok(), log_val)();
        assert_eq!(res, Ok(100));
    }

    #[test]
    fn test_ref_tap_err() {
        let res: Result<i32, &str> = Err("fail").tap_cond(|x| x.err(), log_str)();
        assert_eq!(res.err(), Some("fail"));
        assert_eq!(res, Err("fail"));
    }

    #[test]
    fn test_mut_ref_tap_some() {
        let mut val = Some(5);
        let _ = (&mut val).tap_cond(|x: &mut &mut Option<i32>| x.as_mut(), mutate_val)();
        assert_eq!(val, Some(15));
    }

    #[test]
    fn test_ref_tap_dbg_style() {
        fn check_ref(v: &&i32) {
            assert_eq!(**v, 500);
        }
        let value = 500;

        let _ = (&value).tap_cond(
            |x: &&i32| {
                #[cfg(debug_assertions)]
                {
                    Some(x)
                }
                #[cfg(not(debug_assertions))]
                {
                    None
                }
            },
            check_ref,
        )();

        assert_eq!(value, 500);
    }
}

#[cfg(test)]
mod mutation_tests {
    use super::*;

    struct Counter {
        count: i32,
    }

    struct Wrapper {
        inner: Counter,
    }

    fn increment(c: &mut Counter) {
        c.count += 1;
    }

    fn add_ten(val: &mut i32) {
        *val += 10;
    }

    #[test]
    fn tap_cond_mutate_struct() {
        let mut wrapper = Wrapper {
            inner: Counter { count: 0 },
        };

        let res = (&mut wrapper).tap_cond(|w| Some(&mut w.inner), increment)();

        assert_eq!(res.inner.count, 1);
    }

    #[test]
    fn tap_proj_mutate_struct() {
        let mut wrapper = Wrapper {
            inner: Counter { count: 0 },
        };

        let res = (&mut wrapper).tap_proj(|w| &mut w.inner, increment)();

        assert_eq!(res.inner.count, 1);
    }

    #[test]
    fn tap_cond_mutate_primitive_field() {
        let mut counter = Counter { count: 5 };

        let res = (&mut counter).tap_cond(|c| Some(&mut c.count), add_ten)();

        assert_eq!(res.count, 15);
    }

    #[test]
    fn tap_proj_mutate_primitive_field() {
        let mut counter = Counter { count: 5 };

        let res = (&mut counter).tap_proj(|c| &mut c.count, add_ten)();

        assert_eq!(res.count, 15);
    }

    #[test]
    fn tap_cond_conditional_mutation() {
        let value = 100;

        fn conditional_proj(v: &mut i32) -> Option<&mut i32> {
            if *v > 50 {
                Some(v)
            } else {
                None
            }
        }

        let res = value.tap_cond(conditional_proj, add_ten)();

        assert_eq!(res, 110);
    }

    #[test]
    fn tap_cond_owned_to_mut_projection() {
        let counter = Counter { count: 0 };

        let res = counter.tap_cond(|c: &mut Counter| Some(&mut c.count), add_ten)();

        assert_eq!(res.count, 10);
    }

    #[test]
    fn tap_proj_owned_to_mut_projection() {
        let counter = Counter { count: 0 };

        let res = counter.tap_proj(|c: &mut Counter| &mut c.count, add_ten)();

        assert_eq!(res.count, 10);
    }
}
#[cfg(test)]
mod fn_bound_tests {
    use super::*;

    struct Token<'a> {
        dropped: &'a mut bool,
        n: i32,
    }
    impl<'a> Drop for Token<'a> {
        fn drop(&mut self) {
            *self.dropped = true;
        }
    }

    #[derive(Clone, Copy)]
    struct Buf<const N: usize> {
        data: [i32; N],
        len: usize,
    }
    impl<const N: usize> Buf<N> {
        fn new() -> Self {
            Self {
                data: [0; N],
                len: 0,
            }
        }
        fn push(&mut self, x: i32) {
            self.data[self.len] = x;
            self.len += 1;
        }
    }

    #[test]
    fn pipe_imm_closure_is_reusable() {
        fn add(x: &i32, y: i32) -> i32 {
            *x + y
        }
        let add_ten = 10.pipe(add);
        assert_eq!(add_ten(1), 11);
        assert_eq!(add_ten(2), 12);
        assert_eq!(add_ten(3), 13);
    }

    #[test]
    fn pipe_imm_closure_works_in_map() {
        fn mul(x: &i32, y: i32) -> i32 {
            *x * y
        }
        let double = 2.pipe(mul);
        assert_eq!([1, 2, 3].map(double), [2, 4, 6]);
    }

    #[test]
    fn pipe_mut_closure_is_fnmut() {
        fn increment_and_get(x: &mut i32) -> i32 {
            *x += 1;
            *x
        }
        let mut counter = 0.pipe(increment_and_get);
        assert_eq!(counter(), 1);
        assert_eq!(counter(), 2);
        assert_eq!(counter(), 3);
    }

    #[test]
    fn pipe_mut_mutates_captured_copy_not_original() {
        fn push(v: &mut Buf<4>, x: i32) {
            v.push(x);
        }
        let mut original = Buf::<4>::new();
        original.push(1);
        let mut appender = original.pipe(push);
        appender(2);
        appender(3);
    }

    #[test]
    fn pipe_own_consumes_value() {
        fn sum(v: [i32; 3]) -> i32 {
            v[0] + v[1] + v[2]
        }
        let result = [1, 2, 3].pipe(sum)();
        assert_eq!(result, 6);
    }

    #[test]
    fn tap_imm_accepts_fnonce_closure() {
        let mut dropped = false;
        let tok = Token {
            dropped: &mut dropped,
            n: 0,
        };
        let result = 42.tap(|_x: &i32| {
            drop(tok);
        })();
        assert_eq!(result, 42);
        assert!(dropped);
    }

    #[test]
    fn tap_mut_accepts_fnonce_closure() {
        let mut dropped = false;
        let tok = Token {
            dropped: &mut dropped,
            n: 5,
        };
        let result = 10.tap(move |x: &mut i32| {
            *x += tok.n;
            drop(tok);
        })();
        assert_eq!(result, 15);
        assert!(dropped);
    }

    #[test]
    fn tap_mut_still_works_with_fn() {
        fn double(x: &mut i32) {
            *x *= 2;
        }
        let result = 5.tap(double)();
        assert_eq!(result, 10);
    }

    #[test]
    fn tap_cond_none_does_not_run_side_effect() {
        let mut ran = false;
        let none: Option<i32> = None;
        let result = none.tap_cond(|x: &Option<i32>| x.as_ref(), {
            let f = |_v: &i32| ran = true;
            f
        })();
        assert_eq!(result, None);
        assert!(!ran);
    }

    #[test]
    fn tap_cond_some_does_run_side_effect() {
        let mut ran = false;
        let some = Some(7);
        let result = some.tap_cond(|x: &Option<i32>| x.as_ref(), {
            let f = |_v: &i32| ran = true;
            f
        })();
        assert_eq!(result, Some(7));
        assert!(ran);
    }

    #[test]
    fn tap_cond_mut_none_skips_mutation() {
        let mut ran = false;
        let val = 3;
        let result = val.tap_cond(|x: &mut i32| if *x > 100 { Some(x) } else { None }, {
            let f = |_v: &mut i32| ran = true;
            f
        })();
        assert_eq!(result, 3);
        assert!(!ran);
    }

    #[test]
    fn tap_cond_mut_accepts_fnonce_projection_and_effect() {
        let mut dropped = false;
        let tok = Token {
            dropped: &mut dropped,
            n: 0,
        };
        let result = Some(10).tap_cond(
            move |x: &mut Option<i32>| {
                let _ = tok.n;
                drop(tok);
                x.as_mut()
            },
            {
                let f = |v: &mut i32| *v += 1;
                f
            },
        )();
        assert_eq!(result, Some(11));
        assert!(dropped);
    }

    #[test]
    fn tap_cond_mut_extra_args() {
        fn add_n(v: &mut i32, n: i32) {
            *v += n;
        }
        let result = 10.tap_cond(|x: &mut i32| if *x >= 0 { Some(x) } else { None }, {
            let f = add_n;
            f
        })(5);
        assert_eq!(result, 15);
    }

    #[test]
    fn test_readme_proj() {
        #[derive(Debug)]
        struct Request<'a> {
            _url: &'a str,
            attempts: u32,
        }

        fn track_retry(count: &mut u32) {
            *count += 1
        }
        fn log_trace<T: core::fmt::Debug>(_val: &T, _label: &str) { /* ... */
        }

        let mut req = Request {
            _url: "https://pipei.rs".into(),
            attempts: 3,
        };

        (&mut req).tap_proj(|r| &mut r.attempts, track_retry)();

        assert_eq!(req.attempts, 4);

        // tap_cond: tap only on Err
        let res = Err::<(), _>(503).tap_cond(|x| x.as_ref().err(), log_trace)("request failed");

        assert_eq!(res.unwrap_err(), 503);

        // tap_cond: tap only in debug builds
        let req = req.tap_cond(
            |r| {
                #[cfg(debug_assertions)]
                {
                    Some(r)
                }
                #[cfg(not(debug_assertions))]
                {
                    None
                }
            },
            log_trace,
        )("FINAL");

        assert_eq!(req.attempts, 4);
    }
}
