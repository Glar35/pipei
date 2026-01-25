#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]

//! # pipei
//!
//! A zero-cost library for composing function calls into fluent pipelines with precise lifetime control.
//!
//! Intuitively, the `.pipe()` operator transforms a function `f(x, y, z)` into a method call `x.pipe(f)(y, z)`.
//!
//! ## Core API
//!
//! * **[`Pipe::pipe`]:** Transforms the value and returns the **new** value.
//! * **[`PipeRef::pipe_ref`]:** Starts a pipe from a mutable reference to derive a borrowed value.
//! * **[`Tap::tap`]:** Runs a side-effect (logging, mutation) and returns the **original** value.
//! * **[`TapWith::tap_with`]:** Projects the value (e.g., gets a field), runs a side-effect on the projection, and returns the **original** value.
//!
//!
//! ```rust
//! # use crate::pipei::Pipe;
//! fn add(a: i32, b: i32) -> i32 { a + b }
//!
//! // Correct:
//! // 10i32.pipe(add)(5);
//!
//! // Incorrect (function is prepared but never called):
//! // 10i32.pipe(add);
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
    type Curry<'a> where Self: 'a, A0: 'a;
    fn curry<'a>(self, arg0: A0) -> Self::Curry<'a>;
}

/// Internal mechanism: Prepares a step with a projection.
pub trait ImplCurryWith<const ARITY: usize, Args, AState, PState, RState, A0: ?Sized, P, T: ?Sized, R: ?Sized> {
    type Curry<'a> where Self: 'a, A0: 'a, P: 'a;
    fn curry_with<'a>(self, arg0: A0, proj: P) -> Self::Curry<'a>;
}

/// Internal mechanism: Prepares a step starting specifically from `&'a mut A0`.
pub trait ImplCurryRef<const ARITY: usize, Args, AState, RState, A0: ?Sized, R: ?Sized> {
    type Curry<'a> where Self: 'a, A0: 'a;
    fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a>;
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

/// Extension trait for safe re-borrowing chains.
pub trait PipeRef<const ARITY: usize, AState, RState> {
    /// Transforms a mutable reference into a borrowed value (or sub-reference).
    ///
    /// This is useful for drilling down into a data structure without taking ownership.
    ///
    /// # Example
    /// ```rust
    /// # use crate::pipei::PipeRef;
    /// fn first_mut(arr: &mut [i32; 3]) -> &mut i32 { &mut arr[0] }
    ///
    /// let mut data = [10, 20, 30];
    /// *data.pipe_ref(first_mut)() = 99;
    /// assert_eq!(data[0], 99);
    /// ```
    #[inline(always)]
    fn pipe_ref<'a, R, F, Args>(&'a mut self, f: F) -> F::Curry<'a>
    where
        F: ImplCurryRef<ARITY, Args, AState, RState, Self, R>,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, AState, RState, T> PipeRef<ARITY, AState, RState> for T {}

/// Extension trait for running side effects without altering the pipeline value.
pub trait Tap<const ARITY: usize, AState, RState> {
    /// Runs a side-effect and returns the original value.
    ///
    /// Supports both immutable inspection and mutable modification of the value.
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
        F: ImplCurry<ARITY, Args, AState, RState, TapMark, Self, R>,
        Self: Sized,
    {
        f.curry(self)
    }
}
impl<const ARITY: usize, AState, RState, T> Tap<ARITY, AState, RState> for T {}

/// Extension trait for running side effects on a projection of the value.
pub trait TapWith<const ARITY: usize, AState, PState, RState> {
    /// Projects the value, runs a side-effect, and returns the original value.
    ///
    /// Useful for focusing on a specific field for validation or modification.
    ///
    /// # Example
    /// ```rust
    /// # use crate::pipei::TapWith;
    /// struct Config { id: i32 }
    /// fn check(id: &i32) { assert!(*id > 0); }
    ///
    /// let c = Config { id: 10 };
    /// // Explicit type often required to distinguish between mutable/immutable source paths
    /// c.tap_with(|c: &Config| &c.id, check)();
    /// ```
    #[inline(always)]
    fn tap_with<'a, R, F, P, T: ?Sized, Args>(self, proj: P, f: F) -> F::Curry<'a>
    where
        F: ImplCurryWith<ARITY, Args, AState, PState, RState, Self, P, T, R>,
        Self: Sized,
    {
        f.curry_with(self, proj)
    }
}
impl<const ARITY: usize, AState, PState, RState, T> TapWith<ARITY, AState, PState, RState> for T {}


// ============================================================================================
// Macro Logic
// ============================================================================================

macro_rules! impl_arity {
    ($N:literal, $feat:literal, [ $($Args:ident),* ], $TupleType:ty) => {
        const _: () = {
            #[cfg(feature = $feat)]
            use crate::{Imm, ImplCurry, ImplCurryWith, ImplCurryRef, Mut, Own, PipeMark, TapMark};

            #[cfg(feature = $feat)]
            // --- Tap (Direct) ---
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Imm, Own, TapMark, A0, R> for F
            where F: for<'b> FnOnce(&'b A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| { self(&arg0, $($Args),*); arg0 }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Mut, Own, TapMark, A0, R> for F
            where F: for<'b> FnOnce(&'b mut A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, mut arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| { self(&mut arg0, $($Args),*); arg0 }
                }
            }

            // --- Tap With (Projection) ---
            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> ImplCurryWith<$N, $TupleType, Imm, Imm, Own, A0, P, T, R> for F
            where
                P: for<'b> FnOnce(&'b A0) -> &'b T,
                F: for<'b> FnOnce(&'b T, $($Args),*) -> R
            {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a, P: 'a;
                #[inline(always)] fn curry_with<'a>(self, arg0: A0, proj: P) -> Self::Curry<'a> {
                    move |$($Args),*| { self(proj(&arg0), $($Args),*); arg0 }
                }
            }

            #[cfg(feature = $feat)]
            impl<F, P, A0, T: ?Sized, $($Args,)* R> ImplCurryWith<$N, $TupleType, Mut, Mut, Own, A0, P, T, R> for F
            where
                P: for<'b> FnOnce(&'b mut A0) -> &'b mut T,
                F: for<'b> FnOnce(&'b mut T, $($Args),*) -> R
            {
                type Curry<'a> = impl FnOnce($($Args),*) -> A0 where F: 'a, A0: 'a, P: 'a;
                #[inline(always)] fn curry_with<'a>(self, mut arg0: A0, proj: P) -> Self::Curry<'a> {
                    move |$($Args),*| { self(proj(&mut arg0), $($Args),*); arg0 }
                }
            }

            // --- Pipe (Direct) ---
            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Imm, Own, PipeMark, A0, R> for F
            where F: for<'b> FnOnce(&'b A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0, $($Args,)* R> ImplCurry<$N, $TupleType, Mut, Own, PipeMark, A0, R> for F
            where F: for<'b> FnOnce(&'b mut A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, mut arg0: A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&mut arg0, $($Args),*)
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

            // --- PipeRef (Direct) ---
            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R: ?Sized> ImplCurryRef<$N, $TupleType, Imm, Imm, A0, R> for F
            where F: for<'b> FnOnce(&'b A0, $($Args),*) -> &'b R {
                type Curry<'a> = impl FnOnce($($Args),*) -> &'a R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&*arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R: ?Sized> ImplCurryRef<$N, $TupleType, Mut, Imm, A0, R> for F
            where F: for<'b> FnOnce(&'b mut A0, $($Args),*) -> &'b R {
                type Curry<'a> = impl FnOnce($($Args),*) -> &'a R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&mut *arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R: ?Sized> ImplCurryRef<$N, $TupleType, Imm, Mut, A0, R> for F
            where F: for<'b> FnOnce(&'b A0, $($Args),*) -> &'b mut R {
                type Curry<'a> = impl FnOnce($($Args),*) -> &'a mut R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&*arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R: ?Sized> ImplCurryRef<$N, $TupleType, Mut, Mut, A0, R> for F
            where F: for<'b> FnOnce(&'b mut A0, $($Args),*) -> &'b mut R {
                type Curry<'a> = impl FnOnce($($Args),*) -> &'a mut R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&mut *arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R> ImplCurryRef<$N, $TupleType, Imm, Own, A0, R> for F
            where F: for<'b> FnOnce(&'b A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&*arg0, $($Args),*)
                }
            }

            #[cfg(feature = $feat)]
            impl<F, A0: ?Sized, $($Args,)* R> ImplCurryRef<$N, $TupleType, Mut, Own, A0, R> for F
            where F: for<'b> FnOnce(&'b mut A0, $($Args),*) -> R {
                type Curry<'a> = impl FnOnce($($Args),*) -> R where F: 'a, A0: 'a;
                #[inline(always)] fn curry<'a>(self, arg0: &'a mut A0) -> Self::Curry<'a> {
                    move |$($Args),*| self(&mut *arg0, $($Args),*)
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

// Generate implementations for Arity 0..100
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
        fn add_one(x: i32) -> i32 { x + 1 }
        assert_eq!(1.pipe(add_one)(), 2);
    }

    #[test]
    #[cfg(feature = "1")]
    fn test_pipe_arity() {
        fn sub(x: i32, y: i32) -> i32 { x - y }
        assert_eq!(10.pipe(sub)(4), 6);
    }

    #[test]
    fn test_tap_with_immutable() {
        struct Container {
            val: i32
        }
        fn check_val(v: &i32) {
            assert_eq!(*v, 10);
        }

        let c = Container { val: 10 };
        // Explicit typing needed to resolve ambiguity between Imm and Mut source paths
        let res = c.tap_with(|x: &Container| &x.val, check_val)();
        assert_eq!(res.val, 10);
    }

    #[test]
    fn test_tap_with_mutable() {
        struct Container {
            val: i32
        }
        fn add_one(v: &mut i32) {
            *v += 1;
        }

        let c = Container { val: 10 };
        let res = c.tap_with(|x| &mut x.val, add_one)();
        assert_eq!(res.val, 11);
    }

    #[test]
    fn test_pipe_ref_mutable_borrow() {
        let mut data = [10, 20, 30];
        fn first_mut(slice: &mut [i32; 3]) -> &mut i32 {
            &mut slice[0]
        }

        let f: &mut i32 = data.pipe_ref(first_mut)();
        *f = 99;
        assert_eq!(data[0], 99);
    }

    #[test]
    fn test_chaining_workflow() {
        fn add(x: i32, y: i32) -> i32 { x + y }
        fn double(x: i32) -> i32 { x * 2 }

        let res = 10
            .pipe(add)(5)   // 15
            .pipe(double)() // 30
            .tap(|x: &i32| assert_eq!(*x, 30))();

        assert_eq!(res, 30);
    }

    #[test]
    fn test_mutable_tap_chain() {
        struct State {
            count: i32
        }
        let s = State { count: 0 };

        let res = s
            .tap(|s: &mut State| s.count += 1)()
            .tap(|s: &mut State| s.count += 2)();

        assert_eq!(res.count, 3);
    }


    #[test]
    fn bound_method_as_callback() {
        struct Button { id: usize }
        impl Button {
            fn on_click(&self, prime: usize) -> usize { self.id % prime }
        }

        let buttons = [Button { id: 5}, Button { id: 6 }];

        // 1. Make the array mutable and wrap items in Option
        let callbacks: [Option<_>; 2] = core::array::from_fn(|i| {
            Some((&buttons[i]).pipe(Button::on_click))
        });

        for (cb, res) in callbacks.into_iter().zip([2, 0]) {
            let cb = cb.unwrap();
            assert_eq!(cb(3), res);
        }
    }

    #[test]
    fn unboxed_bound_methods() {
        struct Threshold(i32);
        impl Threshold {
            fn check(&self, val: i32) -> bool { val > self.0 }
        }

        let low = Threshold(10);
        let high = Threshold(50);

        let mut validators = [
            Some(low.pipe(Threshold::check)),
            Some(high.pipe(Threshold::check)),
        ];

        assert_eq!(validators[0].take().unwrap()(20), true);
        assert_eq!(validators[1].take().unwrap()(20), false);
    }
}