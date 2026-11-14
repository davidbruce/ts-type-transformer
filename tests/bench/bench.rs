// The benchmark through the Rust bindings.
#![no_main]
use bindings::*;
use std::cell::Cell;
use std::rc::Rc;

#[unsafe(no_mangle)]
pub extern "C" fn calls(n: i32) -> f64 {
    let mut s = 0.0;
    for i in 0..n {
        s = add(s, i as f64);
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn objects(n: i32) -> f64 {
    let mut s = 0.0;
    for i in 0..n {
        s += Vec2::new(i as f64, 1.0).length();
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn properties(n: i32) -> f64 {
    let v = Vec2::new(0.0, 0.0);
    let mut s = 0.0;
    for i in 0..n {
        v.set_x(i as f64);
        s += v.x();
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn strings_in(n: i32) -> f64 {
    let mut s = 0.0;
    for _ in 0..n {
        s += count("hello world");
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn strings_fresh(n: i32) -> f64 {
    let mut s = 0.0;
    let mut text = *b"hello world";
    for i in 0..n {
        text[0] = b'a' + (i % 26) as u8;
        s += count(std::str::from_utf8(&text).unwrap());
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn strings_out(n: i32) -> f64 {
    let mut s = 0.0;
    for _ in 0..n {
        let g = greet("you");
        s += (g.len() + g.as_bytes()[0] as usize) as f64;
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn callbacks(n: i32) -> f64 {
    let s = Rc::new(Cell::new(0.0));
    let f = EachF::new({
        let s = s.clone();
        move |i: f64| s.set(s.get() + i)
    });
    each(n as f64, &f);
    f.release();
    s.get()
}

#[unsafe(no_mangle)]
pub extern "C" fn compute(n: i32) -> f64 {
    let mut x: u32 = 1;
    let mut s = 0.0;
    for _ in 0..n {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        s += (x >> 24) as f64;
    }
    s
}
