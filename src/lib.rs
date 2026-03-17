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
    /// let mut req = Request { url: "https://pipei.rs".into(), attempts: 3 };
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
