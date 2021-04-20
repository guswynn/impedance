// TODO(guswynn): bencher? criterion?
#![feature(test)]
extern crate test;

use futures::stream::{iter, StreamExt};
use impedance::adaptive::{AdaptiveFuture, Token};
use once_cell::sync::Lazy;
use std::future::Future;
use test::{black_box, Bencher};

fn slow(idx: usize) -> usize {
    println!("{}", idx);
    std::thread::sleep(std::time::Duration::from_millis(10));
    idx
}

fn fast(idx: usize) -> usize {
    println!("{}", idx);
    idx
}

// TODO(guswynn): can I make this not need to pass usize to W?
fn benchmark<B: Fn(usize) -> usize + Copy, W: Fn(usize, B) -> F + Copy, F: Future>(
    b: &mut Bencher,
    bench: B,
    wrapper: W,
) where
    F::Output: std::fmt::Display,
{
    let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    b.iter(|| {
        runtime.block_on(async {
            let v: Vec<usize> = (0..10).collect();
            println!("new one");
            let mut stream = iter(v)
                .map(|i| async move { black_box(wrapper(i, bench).await) })
                .buffer_unordered(10);

            while let Some(i) = stream.next().await {
                println!("got: {}", i);
                black_box(i);
            }
            println!("HUH");
        });
    });
}

static TOKEN: Lazy<Token> = Lazy::new(|| Token::new());

#[bench]
fn slow_with_adaptive(b: &mut Bencher) {
    benchmark(b, slow, |i, f| AdaptiveFuture::new(*TOKEN, move || f(i)));
}

#[bench]
fn slow_with_spawn_blocking(b: &mut Bencher) {
    benchmark(b, slow, |i, f| async move {
        tokio::task::spawn_blocking(move || f(i)).await.unwrap()
    });
}

#[bench]
fn slow_with_nothing(b: &mut Bencher) {
    benchmark(b, slow, |i, f| async move { f(i) });
}

#[bench]
fn slow_with_spawn(b: &mut Bencher) {
    benchmark(b, slow, |i, f| async move {
        tokio::task::spawn(async move { f(i) }).await.unwrap()
    });
}

#[bench]
fn fast_with_adaptive(b: &mut Bencher) {
    benchmark(b, fast, |i, f| AdaptiveFuture::new(*TOKEN, move || f(i)));
}

#[bench]
fn fast_with_spawn_blocking(b: &mut Bencher) {
    benchmark(b, fast, |i, f| async move {
        tokio::task::spawn_blocking(move || f(i)).await.unwrap()
    });
}

#[bench]
fn fast_with_nothing(b: &mut Bencher) {
    benchmark(b, fast, |i, f| async move { f(i) });
}

#[bench]
fn fast_with_adaptive_always_inline(b: &mut Bencher) {
    benchmark(b, fast, |i, f| {
        AdaptiveFuture::new(Token::always_inline(), move || f(i))
    });
}

#[bench]
fn fast_with_adaptive_always_spawn(b: &mut Bencher) {
    benchmark(b, fast, |i, f| {
        AdaptiveFuture::new(Token::always_spawn(), move || f(i))
    });
}
