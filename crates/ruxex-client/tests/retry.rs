use http::HeaderMap;
use http::HeaderValue;
use http::Method;
use http::StatusCode;
use ruxex_client::Request;
use ruxex_client::RetryOn;
use ruxex_client::RetryPolicy;
use ruxex_client::TransportError;
use ruxex_client::run_with_retry;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn run_with_retry_honors_retry_after_without_restarting_the_deadline() {
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let attempts_for_op = Arc::clone(&attempts);
    let mut headers = HeaderMap::new();
    headers.insert("Retry-After", HeaderValue::from_static("10"));

    let task = tokio::spawn(run_with_retry(
        RetryPolicy {
            max_attempts: 1,
            base_delay: Duration::from_secs(1),
            retry_on: RetryOn {
                retry_429: false,
                retry_5xx: true,
                retry_transport: false,
            },
        },
        || Request::new(Method::POST, "http://localhost/v1/responses".into()),
        move |_, attempt| {
            let attempts = Arc::clone(&attempts_for_op);
            let headers = headers.clone();
            async move {
                attempts
                    .lock()
                    .expect("retry attempts should not be poisoned")
                    .push((attempt, tokio::time::Instant::now()));
                if attempt == 0 {
                    Err(TransportError::Http {
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        url: None,
                        headers: Some(headers),
                        body: None,
                    })
                } else {
                    Ok(())
                }
            }
        },
    ));

    tokio::task::yield_now().await;
    assert_eq!(attempts.lock().unwrap().len(), 1);

    tokio::time::advance(Duration::from_secs(9)).await;
    tokio::task::yield_now().await;
    assert_eq!(attempts.lock().unwrap().len(), 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(task.await.expect("retry task should join").unwrap(), ());

    let attempts = attempts.lock().unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].0, 0);
    assert_eq!(attempts[1].0, 1);
    assert_eq!(attempts[1].1 - attempts[0].1, Duration::from_secs(10));
}
