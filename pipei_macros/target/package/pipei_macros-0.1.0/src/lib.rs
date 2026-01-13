// pipei_macros/src/lib.rs

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, LitInt};

#[proc_macro]
pub fn pipei_traits(input: TokenStream) -> TokenStream {
    let i_lit = parse_macro_input!(input as LitInt);
    let i: usize = match i_lit.base10_parse() {
        Ok(v) if v >= 1 => v,
        _ => {
            return quote!(
                compile_error!("pipei_traits!(i): i must be an integer literal >= 1");
            )
                .into();
        }
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

    // Method-level docs with the *numeric* i substituted into a1..ai, A1..Ai, etc.
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

    let out = quote! {
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
    };

    out.into()
}
