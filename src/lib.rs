#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]

//! # pipei
//!
//! A zero-cost library for chaining multi-argument functions using method syntax.
//!
//! It turns a standard function call `f(x, y, z)` into a method call `x.pipe(f)(y, z)`.
//!
//! ## Core API
//!
//! * **[`Pipe::pipe`]:** Passes the value into a function and returns the result.
//! * **[`Tap::tap`]:** Inspects or mutates the value, then returns the original value.
//! * **[`TapWith::tap_with`]:** Inspects or mutates a projection of the value, then returns the original value.
//!
//! ```rust
//! # use crate::pipei::Pipe;
//! fn add(a: i32, b: i32) -> i32 { a + b }
//!
//! // Equivalent to add(10, 5)
//! let result = 10.pipe(add)(5);
//!
//! assert_eq!(result, 15);
//! ```

extern crate alloc;

/// Marker: pass the pipeline value immutably (`&T`).
pub struct Imm;
/// Marker: pass the pipeline value mutably (`&mut T`).
pub struct Mut;
/// Marker: pass/return the pipeline value by value (`T`).
pub struct Own;

/// Marker selecting `tap` semantics (return original value).
pub struct TapMark;
/// Marker selecting `pipe` semantics (return transform result).
pub struct PipeMark;

// ============================================================================================
// Core Traits
// ============================================================================================

/// Internal mechanism: Prepares a step starting from an owned value or direct reference.
pub trait ImplCurry<const ARITY: usize, Args, AState, RState, MARK, A0: ?Sized, R: ?Sized> {
    type Curry<'a>
    where
        Self: 'a,
        A0: 'a;
    fn curry<'a>(self, arg0: A0) -> Self::Curry<'a>;
}

/// Internal mechanism: Prepares a step with a projection.
pub trait ImplCurryWith<const ARITY: usize, Args, State, A0: ?Sized, P, R: ?Sized> {
    type Curry<'a>
    where
        Self: 'a,
        A0: 'a,
        P: 'a;
    fn curry_with<'a>(self, arg0: A0, proj: P) -> Self::Curry<'a>;
}

// ============================================================================================
// Public Extension Traits
// ============================================================================================

/// Extension trait for transforming values.
pub trait Pipe<const ARITY: usize, AState, RState> {
    /// Transforms the value into a new value.
    ///
    /// # Example
    /// ```rust
    /// # use crate::pipei::Pipe;
    /// fn add(a: i32, b: i32) -> i32 { a + b }
    ///
    /// let result = 10i32.pipe(add)(5);
    /// assert_eq!(result, 15);
    /// ```
    #[inline(always)]
    fn pipe<'a, R, F, Args>(self, f: F) -> F::Curry<'a>
    where
        F: ImplCurry<ARITY, Args, AState, RState, PipeMark, Self, R>,
        Self: Sized,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, AState, RState, T> Pipe<ARITY, AState, RState> for T {}

/// Extension trait for running side effects without altering the pipeline value.
pub trait Tap<const ARITY: usize, State> {
    /// Runs a side-effect and returns the original value.
    ///
    /// Supports both immutable and mutable operations on the value.
    ///
    /// # Example
    /// ```rust
    /// # use crate::pipei::Tap;
    /// let x = 10
    ///     .tap(|x: &mut i32, n| *x += n)(5);
    /// assert_eq!(x, 15);
    /// ```
    #[inline(always)]
    fn tap<'a, R, F, Args>(self, f: F) -> F::Curry<'a>
    where
        F: ImplCurry<ARITY, Args, State, Own, TapMark, Self, R>,
        Self: Sized,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, State, T> Tap<ARITY, State> for T {}

/// Extension trait for running side effects on a projection of the value.
pub trait TapWith<const ARITY: usize, State> {
    /// Projects the value into Option, runs a side-effect if Some, and returns the original value.
    ///
    /// Useful for control flow, and for focusing on a specific field for validation or modification.
    ///
    /// # Example
    /// ```rust
    /// # use crate::pipei::TapWith;
    /// struct Config { id: i32 }
    /// fn check(id: &i32) { assert!(*id > 0); }
    ///
    /// let c = Config { id: 10 };
    /// // Explicit type often required to distinguish between mutable/immutable source paths
    /// c.tap_with(|c: &Config| Some(&c.id), check)();
    /// ```
    #[inline(always)]
    fn tap_with<'a, R, F, P, Args>(self, proj: P, f: F) -> F::Curry<'a>
    where
        F: ImplCurryWith<ARITY, Args, State, Self, P, R>,
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
            use crate::{Imm, ImplCurry, ImplCurryWith, Mut, Own, PipeMark, TapMark};

            // --- Pipe ---
            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Imm, Own, PipeMark, A0, R> for F
            where F: for<'b> Fn(&'b A0, $($Args),*) -> R {
                type Curry<'a> = impl Fn($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Own, Own, PipeMark, A0, R> for F
            where F: FnOnce(A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Mut, Own, PipeMark, A0, R> for F
            where F: for<'b> FnMut(&'b mut A0, $($Args),*) -> R {
                type Curry<'a> = impl FnMut($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(mut self, mut arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&mut arg0, $($Args),*)
                }
            }

            // --- Tap ---
            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Imm, Own, TapMark, A0, R> for F
            where F: FnOnce(& A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| { self(&arg0, $($Args),*); arg0 }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Mut, Own, TapMark, A0, R> for F
            where F: FnMut(&mut A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(mut self, mut arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| { self(&mut arg0, $($Args),*); arg0 }
                }
            }

            // --- Tap With (Projection) ---
            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> ImplCurryWith<$N, $TupleType, Imm, A0, P, R> for F
            where
                P: for<'b> FnOnce(&'b A0) -> Option<&'b T>,
                F: FnOnce(&T, $($Args),*) -> R
            {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a, P: 'a;
                #[inline(always)] fn curry_with<'a>(self, arg0: A0, proj: P) -> Self::Curry<'a> {
                    move |$($Args),*| {
                        if let Some(v) = proj(&arg0) { self(v, $($Args),*); }
                        arg0
                    }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> ImplCurryWith<$N, $TupleType, Mut, A0, P, R> for F
            where
                P: for<'b> FnMut(&'b mut A0) -> Option<&'b mut T>,
                F: FnMut(& mut T, $($Args),*) -> R
            {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a, P: 'a;
                #[inline(always)] fn curry_with<'a>(mut self, mut arg0: A0, mut proj: P) -> Self::Curry<'a> {
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
    fn test_tap_with_immutable() {
        struct Container {
            val: i32,
        }
        fn check_val(v: &i32) {
            assert_eq!(*v, 10);
        }

        let c = Container { val: 10 };
        // Explicit typing needed to resolve ambiguity between Imm and Mut source paths
        let res = c.tap_with(|x: &Container| Some(&x.val), check_val)();
        assert_eq!(res.val, 10);
    }

    #[test]
    fn test_tap_with_mutable() {
        struct Container {
            val: i32,
        }
        fn add_one(v: &mut i32) {
            *v += 1;
        }

        let c = Container { val: 10 };
        let res = c.tap_with(|x| Some(&mut x.val), add_one)();
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

        (&s).tap_with(|x| Some(x.ip.as_bytes()), check_ipv4)();
        assert_eq!(s.port, 8080);

        let s = s.tap_with(|x| Some(x.ip.as_bytes()), check_ipv4)();
        assert_eq!(s.port, 8080);
    }

    #[test]
    fn tap_extended() {
        fn assert_lt(x: &i32, n: i32) {
            assert!(*x < n)
        }

        let val = 0.tap_with(|x| if *x < 5 { Some(x) } else { None }, assert_lt)(5);
        assert_eq!(val, 0)
    }

    #[test]
    fn tap_extended_simplified() {
        fn assertion(x: &i32) {
            assert!(*x < 5)
        }

        let val = 0.tap_with(|x| Some(x), assertion)();
        assert_eq!(val, 0)
    }
    // // fail case:
    // #[test]
    // fn tap_extended_inline() {
    //
    //     let val = 0.tap_with(|x| Some(x),
    //                          |x: &i32| assert!(*x < 5)
    //     )();
    //     assert_eq!(val, 0)
    // }

    #[test]
    fn tap_extended_mut() {
        fn take(x: &mut i32, n: i32) {
            *x -= n;
        }

        let val = 10.tap_with(|x| if *x >= 5 { Some(x) } else { None }, take)(5);
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
        let res = opt.tap_with(|x: &Option<i32>| x.as_ref(), log_val)();
        assert_eq!(res, Some(42));

        let none: Option<i32> = None;
        let res_none = none.tap_with(|x: &Option<i32>| x.as_ref(), log_val)();
        assert_eq!(res_none, None);
    }

    #[test]
    fn test_simulate_tap_ok() {
        let res: Result<i32, &str> = Ok(100);
        let final_res = res.tap_with(|x: &Result<i32, &str>| x.as_ref().ok(), log_val)();
        assert_eq!(final_res, Ok(100));
    }

    #[test]
    fn test_simulate_tap_err() {
        let res: Result<i32, &str> = Err("critical failure");
        let final_res = res.tap_with(|x: &Result<i32, &str>| x.as_ref().err(), log_str)();
        assert_eq!(final_res, Err("critical failure"));
    }

    #[test]
    fn test_simulate_conditional_mutation() {
        let val = Some(5);
        let res = val.tap_with(|x: &mut Option<i32>| x.as_mut(), mutate_val)();
        assert_eq!(res, Some(15));
    }

    #[test]
    fn test_simulate_tap_dbg() {
        fn my_dbg<T: core::fmt::Debug>(_v: &T) {}
        let value = 500;

        let res = value.tap_with(
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
        let _ = (&opt).tap_with(|x: &&Option<i32>| x.as_ref(), log_val)();
        assert_eq!(opt, Some(42));
    }

    #[test]
    fn test_ref_tap_ok() {
        let res: Result<i32, &str> = Ok(100);
        let _ = (&res).tap_with(|x: &&Result<i32, &str>| x.as_ref().ok(), log_val)();
        assert_eq!(res, Ok(100));
    }

    #[test]
    fn test_ref_tap_err() {
        let res: Result<i32, &str> = Err("fail").tap_with(|x| x.err(), log_str)();
        assert_eq!(res.err(), Some("fail"));
        assert_eq!(res, Err("fail"));
    }

    #[test]
    fn test_mut_ref_tap_some() {
        let mut val = Some(5);
        let _ = (&mut val).tap_with(|x: &mut &mut Option<i32>| x.as_mut(), mutate_val)();
        assert_eq!(val, Some(15));
    }

    #[test]
    fn test_ref_tap_dbg_style() {
        fn check_ref(v: &&i32) {
            assert_eq!(**v, 500);
        }
        let value = 500;

        let _ = (&value).tap_with(
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
mod no_std_tests {
    extern crate alloc;
    use crate::TapWith;
    use alloc::string::String;

    #[derive(Debug)]
    struct Request {
        url: String,
        attempts: u32,
    }

    static mut AUDIT_CALLED: bool = false;
    static mut RETRY_CALLED: bool = false;
    static mut TRACE_CALLED: bool = false;

    fn log_audit(_url: &str, _id: u32) {
        unsafe {
            AUDIT_CALLED = true;
        }
    }

    fn log_retry(_err: &str, _count: u32) {
        unsafe {
            RETRY_CALLED = true;
        }
    }

    fn log_trace(_req: &Request, _label: &str) {
        unsafe {
            TRACE_CALLED = true;
        }
    }

    #[test]
    fn test_tap_with_no_std_workflow() {
        // Reset flags for fresh test run
        unsafe {
            AUDIT_CALLED = false;
            RETRY_CALLED = false;
            TRACE_CALLED = false;
        }

        let req = Request {
            url: String::from("https://api.rs"),
            attempts: 3,
        };

        let _ = (&req).tap_with(|r| Some(r.url.as_str()), log_audit)(101);

        let res: Result<Request, &str> =
            Err("Timeout").tap_with(|r| r.as_ref().err().copied(), log_retry)(req.attempts);

        let final_req = req.tap_with(
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

        // Use matches! to check Result without requiring Debug on the Request type
        assert!(matches!(res, Err("Timeout")));
        assert_eq!(final_req.attempts, 3);

        unsafe {
            assert!(AUDIT_CALLED);
            assert!(RETRY_CALLED);
            #[cfg(debug_assertions)]
            assert!(TRACE_CALLED);
        }
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
    fn tap_with_mutate_struct() {
        let mut wrapper = Wrapper {
            inner: Counter { count: 0 },
        };

        let res = (&mut wrapper).tap_with(|w| Some(&mut w.inner), increment)();

        assert_eq!(res.inner.count, 1);
    }

    #[test]
    fn tap_with_mutate_primitive_field() {
        let mut counter = Counter { count: 5 };

        let res = (&mut counter).tap_with(|c| Some(&mut c.count), add_ten)();

        assert_eq!(res.count, 15);
    }

    #[test]
    fn tap_with_conditional_mutation() {
        let value = 100;

        fn conditional_proj(v: &mut i32) -> Option<&mut i32> {
            if *v > 50 { Some(v) } else { None }
        }

        let res = value.tap_with(conditional_proj, add_ten)();

        assert_eq!(res, 110);
    }

    #[test]
    fn tap_with_owned_to_mut_projection() {
        let counter = Counter { count: 0 };

        let res = counter.tap_with(|c: &mut Counter| Some(&mut c.count), add_ten)();

        assert_eq!(res.count, 10);
    }
}
