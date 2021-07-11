use rand::{Rng};
use std::future::Future;
use crate::{DefaultClient, Client};
use crate::http_adapter::{RestHttpAdapter, RestHttpError};
use crate::default_client::DefaultClientError;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_timer::{delay, delay_for};
use std::time::{Instant, Duration};
use async_recursion::async_recursion;
use rand::rngs::StdRng;
use std::thread::{spawn, sleep};

/// Represents a single retry attempt.
#[derive(Clone)]
pub struct Retry {
    /// The delay (milliseconds) in the request retry attributable to the exponential backoff algorithm.
    exponential_backoff: i32,

    /// The delay (milliseconds) in the request retry attributable to the jitter algorithm.
    jitter_backoff: i32
}

/// A configuration for controlling retryable requests.
pub struct RetryConfiguration<R: Rng> {
    /// The base delay (milliseconds) used to calculate the time before making another request attempt.
    /// This base will be raised to N, where N is the number of retry attempts.
    pub base_delay: i32,

    /// The jitter algorithm used to apply randomness to the retry delay. Defaults to <see cref="RetryJitter.FullJitter"/>
    pub jitter: Box<dyn Fn(&[Retry], i32, &mut R) -> i32 + Send>,

    /// The maximum number of attempts to make before cancelling the request task.
    pub max_attempts: usize,

    /// A callback that is invoked before a new retry attempt is made.
    pub retry_listener: Option<Box<dyn Fn() + Send>>,
}

impl RetryConfiguration<StdRng> {
    pub fn new() -> RetryConfiguration<StdRng> {
        // let jitter = full_jitter::<StdRng>;
        RetryConfiguration {
            base_delay: 500,
            jitter: Box::new(full_jitter),
            max_attempts: 4,
            retry_listener: None
        }
    }
}

pub struct RetryHistory<R: Rng + Send> {
    pub retry_configuration: Arc<Mutex<RetryConfiguration<R>>>,
    pub retries: Arc<Mutex<Vec<Retry>>>,
}

impl<R: Rng + Send> RetryHistory<R> {
    pub fn new(retry_configuration: Arc<Mutex<RetryConfiguration<R>>>) -> RetryHistory<R> {
        RetryHistory {
            retry_configuration: retry_configuration.clone(),
            retries: Arc::new(Mutex::new(vec![])),
        }
    }

    fn new_retry(history: &RetryHistory<R>, rng: &mut R) -> Retry {
        let retries = history.retries.lock().expect("Failed to lock mutex");
        let retry_configuration = history.retry_configuration.lock().expect("Failed to lock mutex");
        let new_count = retries.len() + 1;
        let expo_backoff = retry_configuration.base_delay.pow(new_count as u32);
        let jittered_backoff = (retry_configuration.jitter)(retries.as_ref(), expo_backoff, rng);
        Retry {
            exponential_backoff: expo_backoff,
            jitter_backoff: jittered_backoff
        }
    }
}

/// FullJitter is a Jitter algorithm that selects a random point between now and the next retry time.
fn full_jitter<R: Rng>(_history: &[Retry], delay: i32, rng: &mut R) -> i32 {
    let random: f32 = rng.gen();
    ((delay as f32) * random) as i32
}

type Output<T> = Result<T, <DefaultClient<RestHttpAdapter> as Client>::Error>;

pub async fn backoff<R: Rng + Send>(history: RetryHistory<R>, rng: Arc<Mutex<R>>) -> RetryHistory<R> {
    let new_history = RetryHistory {
        retry_configuration: history.retry_configuration.clone(),
        retries: history.retries.clone(),
    };

    let new_retry = {
        let mut rng = rng.lock().expect("Failed to lock mutex");
        RetryHistory::new_retry(&new_history, &mut rng)
    };
    new_history.retries.lock().expect("Failed to lock mutex").push(new_retry.clone());

    let config = new_history.retry_configuration.clone();
    {
        if let Some(ref cb) = config.lock().expect("Failed to lock mutex").retry_listener {
            cb();
        }
    }

    let (tx, rx) = oneshot::channel();

    spawn(move || {
        sleep(Duration::from_millis(new_retry.jitter_backoff as u64));
        tx.send(())
    });

    rx.await.expect("Failed to receive timeout");

    new_history
}

#[cfg(test)]
mod test {
    use rand::thread_rng;
    use super::*;
    use rand::rngs::ThreadRng;

    #[test]
    fn test() {
        let mut rng = thread_rng();

        let jitter = full_jitter::<ThreadRng>;

        let retry_configuration = RetryConfiguration {
           base_delay: 100,
            jitter: Box::new(jitter),
            max_attempts: 5,
            retry_listener: None,
        };

        let result = (retry_configuration.jitter)(&[], 100, &mut rng);
        assert_eq!(result >= 0, true);
        assert_eq!(result <= 100, true);
    }
}