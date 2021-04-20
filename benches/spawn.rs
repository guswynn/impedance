// TODO(guswynn): bencher? criterion?
#![feature(test)]
extern crate test;

use futures::stream::{iter, StreamExt};
use impedance::{adaptive::AdaptiveFuture, token::Token};
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
) {
    let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    b.iter(|| {
        runtime.block_on(async {
            let mut stream = iter(vec![0; 10])
                .map(|i| async move { black_box(wrapper(i, bench).await) })
                .buffer_unordered(10);

            while let Some(i) = stream.next().await {
                black_box(i);
            }
        });
    });
}

static TOKEN: Lazy<Token> = Lazy::new(|| Token::new());

#[bench]
fn with_adaptive_slow(b: &mut Bencher) {
    benchmark(b, slow, |i, f| AdaptiveFuture::new(*TOKEN, move || f(i)));
}

#[bench]
fn no_adaptive_slow(b: &mut Bencher) {
    benchmark(b, slow, |i, f| async move { f(i) });
}

#[bench]
fn with_adaptive_fast(b: &mut Bencher) {
    benchmark(b, fast, |i, f| AdaptiveFuture::new(*TOKEN, move || f(i)));
}

#[bench]
fn no_adaptive_fast(b: &mut Bencher) {
    benchmark(b, fast, |i, f| async move { f(i) });
}
