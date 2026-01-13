//! # pipei
//!
//! `pipei` provides *pipe-style partial application* combinators for chaining
//! multi-argument **free functions** without writing closures.
//!
//! For each arity `i ≥ 0`, enabling feature `"i"` exports four traits:
//! - `Pipe{i}Ref` — borrow-based pipes (`&self` / `&mut self` projected into the call)
//! - `Pipe{i}`    — by-value pipes (moves `self` into the call)
//! - `Tap{i}Ref`  — borrow-based taps (run for side effects, return a borrow for chaining)
//! - `Tap{i}`     — by-value taps (run for side effects, return `Self` for chaining)
//!
//! ## Intuition
//!
//! `pipe{i}`/`tap{i}` *fix the first argument* of an `(first, A1..Ai) -> R` function to
//! the receiver and return a closure that accepts the remaining `i` arguments.
//!
//! - **Pipe** transforms: returns the function result.
//! - **Tap** inspects/mutates: discards the function result and returns the original receiver
//!   (as `&Self` / `&mut Self` / `Self`, depending on the trait/method used).
//!
//! ## Semantics (schematic)
//!
//! For `i ≥ 1` (arity-0 is the same idea with an empty argument list):
//!
//! ```text
//! // Pipes (return the result of f)
//! self.pipe{i}(f)(a1..ai)                  = f(self,             a1..ai)
//! self.pipe{i}_with(proj, f)(a1..ai)       = f(proj(&self),      a1..ai)
//! self.pipe{i}_with_mut(proj, f)(a1..ai)   = f(proj(&mut self),  a1..ai)
//!
//! // Taps (discard the result of f, return the receiver)
//! self.tap{i}_with(proj, f)(a1..ai)        = { let _ = f(proj(&self),     a1..ai); &self      }
//! self.tap{i}_with_mut(proj, f)(a1..ai)    = { let _ = f(proj(&mut self), a1..ai); &mut self  }
//!
//! // Unified by-value tap: f may take either &Self or &mut Self.
//! self.tap{i}(f)(a1..ai)                   = { let mut s = self; f(&s, a1..ai) or f(&mut s, a1..ai); s }
//! ```
//!
//! Notes:
//! - `Tap{i}Ref::tap{i}_with` always calls an immutable function `f: (&X, ..) -> R` and returns `&Self`.
//! - `Tap{i}Ref::tap{i}_with_mut` accepts **either** `f: (&X, ..) -> R` **or** `f: (&mut X, ..) -> R`
//!   (selected via a marker), and returns `&mut Self`.
//! - `Tap{i}::tap{i}` (by value) similarly accepts either `Fn(&Self, ..)` or `Fn(&mut Self, ..)` and returns `Self`.
//!
//! ## Projections
//!
//! The `_with` / `_with_mut` variants apply a *projection* before calling the function.
//! This is useful for type adaptation or focusing on a component:
//!
//! - `String -> &str` via `|s| s.as_str()`
//! - `Vec<T> -> &[T]` via `|v| v.as_slice()`
//! - `Config -> &u16` via `|c| &c.port`
//!
//! ## Feature gating
//!
//! Only arities whose numeric feature is enabled are compiled and exported
//! (e.g. `"0"`, `"1"`, `"2"`, …). Convenience features like `"up_to_5"` can enable ranges.


#![no_std]

// --- Helper Types for tap{i}---
pub struct Imm;
pub struct Mut;

#[cfg(feature = "0")]
mod pipe_0 {
    //! Arity-0 pipe combinators.

    /// Borrow-based arity-0 piping.
    ///
    /// Fixes the first argument to `self` (possibly projected) and returns a
    /// nullary closure.

    use crate::{Imm, Mut};
    pub trait Pipe0Ref {
        /// Partially applies `f` by fixing its first argument to `proj(&self)`,
        /// returning a nullary closure.
        ///
        /// Law: `self.pipe0_with(proj, f)() = f(proj(&self))`.
        #[inline]
        fn pipe0_with<'a, X: ?Sized + 'a, R: 'a>(
            &'a self,
            proj: impl FnOnce(&'a Self) -> &'a X,
            f: impl FnOnce(&'a X) -> R,
        ) -> impl FnOnce() -> R {
            move || f(proj(self))
        }

        /// Partially applies `f` by fixing its first argument to `proj(&mut self)`,
        /// returning a nullary closure.
        ///
        /// Law: `self.pipe0_with_mut(proj, f)() = f(proj(&mut self))`.
        #[inline]
        fn pipe0_with_mut<'a, X: ?Sized + 'a, R: 'a>(
            &'a mut self,
            proj: impl FnOnce(&'a mut Self) -> &'a mut X,
            f: impl FnOnce(&'a mut X) -> R,
        ) -> impl FnOnce() -> R {
            move || f(proj(self))
        }
    }
    impl<T: ?Sized> Pipe0Ref for T {}

    /// By-value arity-0 piping.
    ///
    /// Moves `self` into the returned nullary closure.
    pub trait Pipe0: Sized + Pipe0Ref {
        /// Partially applies `f` by moving `self` into the returned nullary closure.
        ///
        /// Law: `self.pipe0(f)() = f(self)`.
        #[inline]
        fn pipe0<R>(self, f: impl FnOnce(Self) -> R) -> impl FnOnce() -> R {
            move || f(self)
        }
    }
    impl<T: Sized> Pipe0 for T {}

    // --- Arity 0 Helpers ---

    /// Auxiliary trait to abstract over mutable vs. immutable closures for arity 0.
    pub trait Tap0Fn<Rec: ?Sized, Marker> {
        fn call(self, rec: &mut Rec);
    }

    // Impl for Immutable Fn(&T) -> coerces &mut T to &T
    impl<F, Rec: ?Sized, R> Tap0Fn<Rec, Imm> for F
    where
        F: FnOnce(&Rec) -> R
    {
        #[inline]
        fn call(self, rec: &mut Rec) { (self)(rec); }
    }

    // Impl for Mutable Fn(&mut T) -> passes &mut T directly
    impl<F, Rec: ?Sized, R> Tap0Fn<Rec, Mut> for F
    where
        F: FnOnce(&mut Rec) -> R
    {
        #[inline]
        fn call(self, rec: &mut Rec) { (self)(rec); }
    }

    // --- Arity 0 Traits ---

    pub trait Tap0Ref {
        /// Immutable view tap. Always takes `Fn(&T)`.
        #[inline]
        fn tap0_with<'a, X: ?Sized + 'a, R>(
            &'a self,
            proj: impl FnOnce(&'a Self) -> &'a X,
            f: impl FnOnce(&'a X) -> R,
        ) -> impl FnOnce() -> &'a Self {
            move || {
                let _ = f(proj(self));
                self
            }
        }

        /// Mutable view tap. Accepts BOTH `Fn(&mut T)` and `Fn(&T)`.
        #[inline]
        fn tap0_with_mut<'a, X: ?Sized + 'a, M, F>(
            &'a mut self,
            proj: impl FnOnce(&mut Self) -> &mut X,
            f: F,
        ) -> impl FnOnce() -> &'a mut Self
        where
            F: Tap0Fn<X, M> 
        {
            move || {
                f.call(proj(&mut *self));
                self
            }
        }
    }
    impl<T: ?Sized> Tap0Ref for T {}

    pub trait Tap0: Sized + Tap0Ref {
        /// By-value tap. Accepts BOTH `Fn(&mut T)` and `Fn(&T)`.
        #[inline]
        fn tap0<M, F>(self, f: F) -> impl FnOnce() -> Self
        where
            F: Tap0Fn<Self, M>
        {
            move || {
                let mut s = self;
                f.call(&mut s);
                s
            }
        }
    }
    impl<T: Sized> Tap0 for T {}
}

#[cfg(feature = "0")]
pub use pipe_0::{Pipe0, Pipe0Ref, Tap0, Tap0Ref};

use paste::paste;

/// Generates `Pipe{i}Ref` + `Pipe{i}` and `Tap{i}Ref` + `Tap{i}` modules gated by numeric features.
macro_rules! gen_pipei {
  ($(($i:literal, $feat:literal)),+ $(,)?) => {
    $(
      paste! {
        #[cfg(feature = $feat)]
        #[doc = concat!("Arity-", $feat, " pipe/tap combinators.")]
        mod [<pipe_ $i>] {
          use pipei_macros::{pipei_traits, tapi_traits};
          use crate::{Imm, Mut};
          pipei_traits!($i);
          tapi_traits!($i);
        }

        #[cfg(feature = $feat)]
        pub use [<pipe_ $i>]::{
          [<Pipe $i>], [<Pipe $i Ref>],
          [<Tap $i>],  [<Tap $i Ref>],
        };
      }
    )+
  }
}

// Generate arities 1..=100 (each gated by feature "i").
gen_pipei! {
  (1,"1"), (2,"2"), (3,"3"), (4,"4"), (5,"5"),
  (6,"6"), (7,"7"), (8,"8"), (9,"9"), (10,"10"),
  (11,"11"), (12,"12"), (13,"13"), (14,"14"), (15,"15"),
  (16,"16"), (17,"17"), (18,"18"), (19,"19"), (20,"20"),
  (21,"21"), (22,"22"), (23,"23"), (24,"24"), (25,"25"),
  (26,"26"), (27,"27"), (28,"28"), (29,"29"), (30,"30"),
  (31,"31"), (32,"32"), (33,"33"), (34,"34"), (35,"35"),
  (36,"36"), (37,"37"), (38,"38"), (39,"39"), (40,"40"),
  (41,"41"), (42,"42"), (43,"43"), (44,"44"), (45,"45"),
  (46,"46"), (47,"47"), (48,"48"), (49,"49"), (50,"50"),
  (51,"51"), (52,"52"), (53,"53"), (54,"54"), (55,"55"),
  (56,"56"), (57,"57"), (58,"58"), (59,"59"), (60,"60"),
  (61,"61"), (62,"62"), (63,"63"), (64,"64"), (65,"65"),
  (66,"66"), (67,"67"), (68,"68"), (69,"69"), (70,"70"),
  (71,"71"), (72,"72"), (73,"73"), (74,"74"), (75,"75"),
  (76,"76"), (77,"77"), (78,"78"), (79,"79"), (80,"80"),
  (81,"81"), (82,"82"), (83,"83"), (84,"84"), (85,"85"),
  (86,"86"), (87,"87"), (88,"88"), (89,"89"), (90,"90"),
  (91,"91"), (92,"92"), (93,"93"), (94,"94"), (95,"95"),
  (96,"96"), (97,"97"), (98,"98"), (99,"99"), (100,"100"),
}