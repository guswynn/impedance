use rayon::iter::IntoParallelIterator;
use std::{
    any::Any,
    fmt::{self, Debug},
};

pub async fn par_iter<T, R, F>(t: T, closure: F) -> Result<R, Panicked>
where
    T: IntoParallelIterator + Send + 'static,
    R: Send + 'static,
    F: FnOnce(<T as IntoParallelIterator>::Iter) -> R + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();

    // Rayon turns panic's inside spawn's into aborts by default, but this
    // is overrideable. We take great care to ensure that we won't panic in this closure
    // and panic's inside the user-provided closure are caught
    rayon::spawn(move || {
        // See https://github.com/rayon-rs/rayon/blob/c571f8ffb4f74c8c09b4e1e6d9979b71b4414d07/rayon-core/src/spawn/mod.rs#L75
        // for a justification of this use of AssertUnwindSafe
        /*
        let pass = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            closure(t.into_par_iter())
        }))
        .map_err(|payload| Panicked { payload });*/

        let pass = Ok(closure(t.into_par_iter()));

        let _ = tx.send(pass);
    });

    rx.await.unwrap()
}

pub trait SetupHelper<'a, T> {
    type Output: IntoParallelIterator + Send + 'a;
    fn call(self, arg: &'a T) -> Self::Output;
}
impl<'a, D: 'a, F, T: 'a> SetupHelper<'a, T> for F
where
    F: FnOnce(&'a T) -> D,
    D: IntoParallelIterator + Send,
{
    type Output = D;
    fn call(self, arg: &'a T) -> D {
        self(arg)
    }
}

pub async fn par_iter_with_setup<T, R, S, M, F>(t: T, setup: S, closure: F) -> Result<R, Panicked>
where
    T: Send + 'static,
    R: Send + 'static,
    M: IntoParallelIterator + Send,
    for<'a> S: SetupHelper<'a, T, Output = M> + Send + 'static,
    F: FnOnce(<M as IntoParallelIterator>::Iter) -> R + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();

    // Rayon turns panic's inside spawn's into aborts by default, but this
    // is overrideable. We take great care to ensure that we won't panic in this closure
    // and panic's inside the user-provided closure are caught
    rayon::spawn(move || {
        let middle = setup.call(&t);
        // See https://github.com/rayon-rs/rayon/blob/c571f8ffb4f74c8c09b4e1e6d9979b71b4414d07/rayon-core/src/spawn/mod.rs#L75
        // for a justification of this use of AssertUnwindSafe
        let pass = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            closure(middle.into_par_iter())
        }))
        .map_err(|payload| Panicked { payload });

        let _ = tx.send(pass);
    });

    rx.await.unwrap()
}

pub struct Panicked {
    pub payload: Box<dyn Any + Send + 'static>,
}

impl Debug for Panicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panicked")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        thread,
        time::{Duration, Instant},
    };

    #[tokio::test]
    async fn test_happy_path() {
        let v = vec![1, 2, 3];

        use rayon::iter::ParallelIterator;
        let sum: usize = par_iter(v, |iter| iter.sum()).await.unwrap();
        assert_eq!(sum, 6);
    }

    // TODO(guswynn): figure out how to use tokio's time testing stuff to make this not timing
    // based
    #[tokio::test]
    async fn test_actually_async() {
        // github actions only have 2 cores
        let v = vec![1usize, 2];

        use rayon::iter::ParallelIterator;
        // This is a rayon iter that takes at least 1 second to complete
        let par_iter = par_iter(v, |iter| {
            iter.map(|_| thread::sleep(Duration::from_secs(1))).count()
        });
        tokio::pin!(par_iter);

        // This a sleep that should take only 50ms to complete
        let async_sleep = tokio::time::sleep(Duration::from_millis(50));
        tokio::pin!(async_sleep);

        // We poll the 1s rayon iter first, but check that we actually
        // finish the 50ms sleep first, close to 50ms
        let now = Instant::now();
        tokio::select! {
            biased;
            _ = &mut par_iter => {
                assert!(false, "Shouldn't make it here")
            }
            _ = async_sleep => {
                eprintln!("made it here sleep");
                assert!(now.elapsed().as_millis() >= 50);
                assert!(now.elapsed().as_millis() <= 75);
            }
        };

        // Then check that we can await the rest of par_iter, in less than 2 seconds
        // (because we are parallizing the many 1 second durations)
        let count = par_iter.await;
        assert_eq!(count.unwrap(), 2);
        assert!(now.elapsed().as_secs() >= 1);
        assert!(now.elapsed().as_secs() < 2);
    }

    #[tokio::test]
    async fn test_panic_in_iter() {
        let v = vec![1usize, 2, 3];

        use rayon::iter::ParallelIterator;
        let panicked = par_iter(v, |iter| iter.map(|_| panic!("gus")).count())
            .await
            .unwrap_err();

        assert_eq!(
            panicked.payload.downcast_ref::<&'static str>().unwrap(),
            &"gus"
        );
    }

    /*
    #[tokio::test]
    async fn test_panic_in_closure() {
        let v = vec![1usize, 2, 3];

        let panicked = par_iter(v, |_| panic!("gus2")).await.unwrap_err();

        assert_eq!(
            panicked.payload.downcast_ref::<&'static str>().unwrap(),
            &"gus2"
        );
    }*/

    fn setup<'a>(v: &'a Vec<String>) -> Vec<&'a str> {
        v.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
    }

    /*
    #[tokio::test]
    async fn test_setup_borrowing() {
        let v: Vec<String> = vec!["gus".to_string(), "gus2".to_string()];

        use rayon::iter::ParallelIterator;
        let total_len: usize = par_iter_with_setup(v, setup, |iter| iter.map(|s| s.len()).sum())
            .await
            .unwrap();

        assert_eq!(total_len, 7,);
    }*/
}
