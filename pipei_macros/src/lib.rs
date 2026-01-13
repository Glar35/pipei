use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::LitInt;

fn parse_i(input: TokenStream, min: usize, msg: &str) -> Result<usize, TokenStream> {
    let Ok(i_lit) = syn::parse::<LitInt>(input) else {
        return Err(quote!(compile_error!(#msg);).into());
    };
    let Ok(v) = i_lit.base10_parse::<usize>() else {
        return Err(quote!(compile_error!(#msg);).into());
    };
    if v < min {
        return Err(quote!(compile_error!(#msg);).into());
    }
    Ok(v)
}

#[proc_macro]
pub fn pipei_traits(input: TokenStream) -> TokenStream {
    let i: usize = match parse_i(input, 1, "pipei_traits!(i): i must be an integer literal >= 1") {
        Ok(i) => i,
        Err(e) => return e,
    };

    // A1..Ai and a1..ai
    let tys: Vec<_> = (1..=i).map(|k| format_ident!("A{k}")).collect();
    let vars: Vec<_> = (1..=i).map(|k| format_ident!("a{k}")).collect();

    let trait_ref = format_ident!("Pipe{i}Ref");
    let trait_val = format_ident!("Pipe{i}");

    let m_with = format_ident!("pipe{i}_with");
    let m_with_mut = format_ident!("pipe{i}_with_mut");
    let m_val = format_ident!("pipe{i}");

    let doc_ref = format!(
        "Pipe-style helpers for partial application of `(first, A1..A{i}) -> R` functions.\n\n\
         The `pipe{i}_with*` methods fix the first argument to a (possibly projected) borrow of \
         `self` and return a closure that accepts the remaining {i} arguments."
    );
    let doc_val = format!(
        "Adds by-value partial application for arity {i} (requires `Self: Sized`)."
    );

    let m_with_s = m_with.to_string();
    let m_with_mut_s = m_with_mut.to_string();
    let m_val_s = m_val.to_string();

    let doc_with = format!(
        "Partially applies `f` by fixing its first argument to `proj(&self)`, returning a closure \
         that takes (A1..A{i}).\n\n\
         Semantics: `self.{m}(proj, f)(a1..a{i}) == f(proj(&self), a1..a{i})`.",
        i = i,
        m = m_with_s
    );

    let doc_with_mut = format!(
        "Partially applies `f` by fixing its first argument to `proj(&mut self)`, returning a closure \
         that takes (A1..A{i}).\n\n\
         Semantics: `self.{m}(proj, f)(a1..a{i}) == f(proj(&mut self), a1..a{i})`.",
        i = i,
        m = m_with_mut_s
    );

    let doc_val_m = format!(
        "Partially applies `f` by moving `self` into the returned closure. The closure takes \
         (A1..A{i}).\n\n\
         Semantics: `self.{m}(f)(a1..a{i}) == f(self, a1..a{i})`.",
        i = i,
        m = m_val_s
    );

    quote! {
        #[doc = #doc_ref]
        pub trait #trait_ref {
            #[doc = #doc_with]
            #[inline]
            fn #m_with<'x, X: ?Sized + 'x, R: 'x, #(#tys),*>(
                &'x self,
                proj: impl FnOnce(&'x Self) -> &'x X,
                f: impl FnOnce(&'x X, #(#tys),*) -> R,
            ) -> impl FnOnce(#(#tys),*) -> R {
                move |#(#vars),*| f(proj(self), #(#vars),*)
            }

            #[doc = #doc_with_mut]
            #[inline]
            fn #m_with_mut<'x, X: ?Sized + 'x, R: 'x, #(#tys),*>(
                &'x mut self,
                proj: impl FnOnce(&'x mut Self) -> &'x mut X,
                f: impl FnOnce(&'x mut X, #(#tys),*) -> R,
            ) -> impl FnOnce(#(#tys),*) -> R {
                move |#(#vars),*| f(proj(self), #(#vars),*)
            }
        }
        impl<T: ?Sized> #trait_ref for T {}

        #[doc = #doc_val]
        pub trait #trait_val: Sized + #trait_ref {
            #[doc = #doc_val_m]
            #[inline]
            fn #m_val<R, #(#tys),*>(
                self,
                f: impl FnOnce(Self, #(#tys),*) -> R,
            ) -> impl FnOnce(#(#tys),*) -> R {
                move |#(#vars),*| f(self, #(#vars),*)
            }
        }
        impl<T: Sized> #trait_val for T {}
    }
        .into()
}

#[proc_macro]
pub fn tapi_traits(input: TokenStream) -> TokenStream {
    let i = match parse_i(input, 1, "tapi_traits!(i): i must be an integer literal >= 1") {
        Ok(i) => i,
        Err(e) => return e,
    };

    let tys: Vec<_> = (1..=i).map(|k| format_ident!("A{k}")).collect();
    let vars: Vec<_> = (1..=i).map(|k| format_ident!("a{k}")).collect();

    let trait_ref = format_ident!("Tap{i}Ref");
    let trait_val = format_ident!("Tap{i}");
    let trait_fn = format_ident!("Tap{i}Fn"); // Helper trait for overloading

    let m_with = format_ident!("tap{i}_with");
    let m_with_mut = format_ident!("tap{i}_with_mut");

    let m_val = format_ident!("tap{i}"); // Unified method

    let doc_ref = format!(
        "Tap-style helpers for calling `(first, A1..A{i}) -> R` for side effects.\n\n\
         The `tap{i}_with*` methods call `f` with a (possibly projected) borrow of `self`, \
         discard `R`, and return `self` to enable chaining."
    );
    let doc_val = format!(
        "Adds by-value tap helpers for arity {i} (requires `Self: Sized`)."
    );

    let doc_with = format!(
        "Calls `f(proj(&self), a1..a{i})` (immutable view), discards the return value, and returns `&self`.",
        i = i
    );

    let doc_with_mut = format!(
        "Calls `f(proj(&mut self), a1..a{i})` (mutable view), discards the return value, and returns `&mut self`.\n\n\
         This method accepts both immutable `Fn(&X)` and mutable `Fn(&mut X)` functions.",
        i = i
    );

    let doc_val_unified = format!(
        "Moves `self` in, calls `f` (which can accept `&self` OR `&mut self`), and returns `self`."
    );

    quote! {
        // --- Helper Trait for Overloading ---

        /// Auxiliary trait to abstract over mutable vs. immutable closures.
        /// (Internal usage for this arity).
        pub trait #trait_fn<Rec: ?Sized, #(#tys,)* Marker> {
            fn call(self, rec: &mut Rec, #(#vars: #tys),*);
        }

        // 1. Immutable Implementation (Fn(&T))
        // Uses `Imm` (assumed to exist in lib.rs scope)
        impl<F, Rec: ?Sized, R, #(#tys),*> #trait_fn<Rec, #(#tys,)* Imm> for F
        where
            F: FnOnce(&Rec, #(#tys),*) -> R,
        {
            #[inline]
            fn call(self, rec: &mut Rec, #(#vars: #tys),*) {
                // Coerce &mut Rec -> &Rec
                (self)(rec, #(#vars),*);
            }
        }

        // 2. Mutable Implementation (Fn(&mut T))
        // Uses `Mut` (assumed to exist in lib.rs scope)
        impl<F, Rec: ?Sized, R, #(#tys),*> #trait_fn<Rec, #(#tys,)* Mut> for F
        where
            F: FnOnce(&mut Rec, #(#tys),*) -> R,
        {
            #[inline]
            fn call(self, rec: &mut Rec, #(#vars: #tys),*) {
                // Pass &mut Rec directly
                (self)(rec, #(#vars),*);
            }
        }

        // --- Reference Trait ---

        #[doc = #doc_ref]
        pub trait #trait_ref {
            #[doc = #doc_with]
            #[inline]
            fn #m_with<'x, X: ?Sized + 'x, R, #(#tys),*>(
                &'x self,
                proj: impl FnOnce(&'x Self) -> &'x X,
                f: impl FnOnce(&'x X, #(#tys),*) -> R,
            ) -> impl FnOnce(#(#tys),*) -> &'x Self {
                move |#(#vars),*| {
                    let _ = f(proj(self), #(#vars),*);
                    self
                }
            }

            #[doc = #doc_with_mut]
            #[inline]
            fn #m_with_mut<'x, X: ?Sized + 'x, M, F, #(#tys),*>(
                &'x mut self,
                proj: impl FnOnce(&mut Self) -> &mut X,
                f: F,
            ) -> impl FnOnce(#(#tys),*) -> &'x mut Self
            where
                F: #trait_fn<X, #(#tys,)* M>
            {
                move |#(#vars),*| {
                    f.call(proj(&mut *self), #(#vars),*);
                    self
                }
            }
        }
        impl<T: ?Sized> #trait_ref for T {}

        // --- Value Trait ---

        #[doc = #doc_val]
        pub trait #trait_val: Sized + #trait_ref {
            #[doc = #doc_val_unified]
            #[inline]
            fn #m_val<M, F, #(#tys),*>(
                self,
                f: F,
            ) -> impl FnOnce(#(#tys),*) -> Self
            where
                F: #trait_fn<Self, #(#tys,)* M>
            {
                move |#(#vars),*| {
                    let mut s = self;
                    f.call(&mut s, #(#vars),*);
                    s
                }
            }
        }
        impl<T: Sized> #trait_val for T {}
    }
        .into()
}