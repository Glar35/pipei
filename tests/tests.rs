use pipei::{Pipe, Tap, TapWith};

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
#[cfg(feature = "0")]
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
#[cfg(feature = "0")]
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
#[cfg(feature = "0")]
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
#[cfg(feature = "0")]
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
#[cfg(feature = "0")]
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
#[cfg(all(feature = "0", feature = "1"))]
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
#[cfg(feature = "0")]
fn test_mutable_tap_chain() {
    struct State {
        count: i32,
    }
    let s = State { count: 0 };

    let res = s.tap(|s: &mut State| s.count += 1)().tap(|s: &mut State| s.count += 2)();

    assert_eq!(res.count, 3);
}

#[test]
#[cfg(feature = "1")]
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
#[cfg(feature = "1")]
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
#[cfg(feature = "0")]
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
#[cfg(feature = "1")]
fn tap_extended() {
    fn assert_lt(x: &i32, n: i32) {
        assert!(*x < n)
    }

    let val = 0.tap_cond(|x| if *x < 5 { Some(x) } else { None }, assert_lt)(5);
    assert_eq!(val, 0)
}

#[test]
#[cfg(feature = "0")]
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
#[cfg(all(feature = "0", feature = "1"))]
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
#[cfg(feature = "1")]
fn tap_extended_mut() {
    fn take(x: &mut i32, n: i32) {
        *x -= n;
    }

    let val = 10.tap_cond(|x| if *x >= 5 { Some(x) } else { None }, take)(5);
    assert_eq!(val, 5)
}

#[test]
#[cfg(feature = "1")]
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

// ============================================================================================
// Extended tap tests
// ============================================================================================

#[cfg(feature = "0")]
mod extended_tap_tests {
    use pipei::TapWith;

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

// ============================================================================================
// Reference tap tests
// ============================================================================================

#[cfg(feature = "0")]
mod reference_tap_tests {
    use pipei::TapWith;

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

// ============================================================================================
// Mutation tests
// ============================================================================================

#[cfg(feature = "0")]
mod mutation_tests {
    use pipei::TapWith;

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

// ============================================================================================
// Fn bound tests
// ============================================================================================

mod fn_bound_tests {
    use pipei::{Pipe, Tap, TapWith};

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
    #[cfg(feature = "1")]
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
    #[cfg(feature = "1")]
    fn pipe_imm_closure_works_in_map() {
        fn mul(x: &i32, y: i32) -> i32 {
            *x * y
        }
        let double = 2.pipe(mul);
        assert_eq!([1, 2, 3].map(double), [2, 4, 6]);
    }

    #[test]
    #[cfg(feature = "0")]
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
    #[cfg(feature = "1")]
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
    #[cfg(feature = "0")]
    fn pipe_own_consumes_value() {
        fn sum(v: [i32; 3]) -> i32 {
            v[0] + v[1] + v[2]
        }
        let result = [1, 2, 3].pipe(sum)();
        assert_eq!(result, 6);
    }

    #[test]
    #[cfg(feature = "0")]
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
    #[cfg(feature = "0")]
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
    #[cfg(feature = "0")]
    fn tap_mut_still_works_with_fn() {
        fn double(x: &mut i32) {
            *x *= 2;
        }
        let result = 5.tap(double)();
        assert_eq!(result, 10);
    }

    #[test]
    #[cfg(feature = "0")]
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
    #[cfg(feature = "0")]
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
    #[cfg(feature = "0")]
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
    #[cfg(feature = "0")]
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
    #[cfg(feature = "1")]
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
    #[cfg(all(feature = "0", feature = "1"))]
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
            _url: "https://pipei.rs",
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

// ============================================================================================
// Higher arity tests (new)
// ============================================================================================

#[cfg(feature = "2")]
mod higher_arity_tests {
    use pipei::{Pipe, Tap, TapWith};

    #[test]
    fn pipe_arity_2() {
        fn clamp(val: i32, lo: i32, hi: i32) -> i32 {
            if val < lo {
                lo
            } else if val > hi {
                hi
            } else {
                val
            }
        }
        assert_eq!(50.pipe(clamp)(0, 100), 50);
        assert_eq!(150.pipe(clamp)(0, 100), 100);
        assert_eq!((-5).pipe(clamp)(0, 100), 0);
    }

    #[test]
    fn tap_arity_2() {
        fn assert_between(x: &i32, lo: i32, hi: i32) {
            assert!(*x >= lo && *x <= hi);
        }
        let val = 42.tap(assert_between)(0, 100);
        assert_eq!(val, 42);
    }

    #[test]
    fn tap_cond_arity_2() {
        fn add_scaled(v: &mut i32, base: i32, factor: i32) {
            *v += base * factor;
        }
        let val = 10.tap_cond(
            |x: &mut i32| if *x > 0 { Some(x) } else { None },
            add_scaled,
        )(3, 4);
        assert_eq!(val, 22);
    }

    #[test]
    fn tap_proj_arity_2() {
        struct Pair {
            _a: i32,
            b: i32,
        }
        fn add_scaled(v: &mut i32, base: i32, factor: i32) {
            *v += base * factor;
        }
        let p = Pair { _a: 1, b: 10 }.tap_proj(|p: &mut Pair| &mut p.b, add_scaled)(2, 5);
        assert_eq!(p.b, 20);
    }
}

#[cfg(feature = "3")]
mod arity_3_tests {
    use pipei::Pipe;

    #[test]
    fn pipe_arity_3() {
        fn weighted_sum(base: i32, a: i32, b: i32, c: i32) -> i32 {
            base + a + b * 2 + c * 3
        }
        assert_eq!(100.pipe(weighted_sum)(1, 2, 3), 114);
    }
}

// ============================================================================================
// Cross-arity chaining tests (new)
// ============================================================================================

#[cfg(all(feature = "0", feature = "1", feature = "2"))]
mod cross_arity_chain_tests {
    use pipei::{Pipe, Tap, TapWith};

    #[test]
    fn mixed_arity_pipeline() {
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        fn clamp(val: i32, lo: i32, hi: i32) -> i32 {
            if val < lo {
                lo
            } else if val > hi {
                hi
            } else {
                val
            }
        }
        fn negate(x: i32) -> i32 {
            -x
        }

        let result = 10.pipe(add)(90) // 100, arity 1
            .pipe(clamp)(0, 50) // 50,  arity 2
        .pipe(negate)() // -50, arity 0
        .tap(|x: &i32| assert_eq!(*x, -50))(); // arity 0

        assert_eq!(result, -50);
    }

    #[test]
    fn tap_proj_then_pipe() {
        struct Acc {
            total: i32,
        }
        fn add_to(v: &mut i32, n: i32) {
            *v += n;
        }
        fn extract(a: Acc) -> i32 {
            a.total
        }

        let result = Acc { total: 0 }.tap_proj(|a: &mut Acc| &mut a.total, add_to)(10) // arity 1
            .tap_proj(|a: &mut Acc| &mut a.total, add_to)(20) // arity 1
        .pipe(extract)(); // arity 0

        assert_eq!(result, 30);
    }

    #[test]
    fn tap_cond_skip_in_chain() {
        fn add(x: i32, y: i32) -> i32 {
            x + y
        }
        fn panic_if_called(_v: &mut i32) {
            panic!("should not be called");
        }

        // The tap_cond projection returns None, so the side effect is skipped.
        let result = 5.pipe(add)(5) // 10
            .tap_cond(
                |x: &mut i32| if *x > 100 { Some(x) } else { None },
                panic_if_called,
            )()
        .pipe(add)(1); // 11

        assert_eq!(result, 11);
    }
}
