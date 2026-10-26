use super::*;
use http::HeaderValue;
use pretty_assertions::assert_eq;

#[tokio::test(start_paused = true)]
async fn retry_after_counts_down_without_restarting_the_deadline() {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("10"));
    let advice = RetryAfter::from_headers(&headers).expect("valid retry advice");

    assert_eq!(advice.remaining_delay(), Duration::from_secs(10));
    tokio::time::advance(Duration::from_secs(4)).await;
    assert_eq!(advice.remaining_delay(), Duration::from_secs(6));
    tokio::time::advance(Duration::from_secs(7)).await;
    assert_eq!(advice.remaining_delay(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn retry_after_accepts_http_dates_and_rejects_invalid_values() {
    let value = httpdate::fmt_http_date(SystemTime::now() + Duration::from_secs(60));
    let advice = RetryAfter::from_header(&value).expect("valid HTTP date");
    assert!(advice.remaining_delay() <= Duration::from_secs(60));

    for value in ["0", "Thu, 01 Jan 1970 00:00:00 GMT"] {
        assert_eq!(
            RetryAfter::from_header(value).map(RetryAfter::remaining_delay),
            Some(Duration::ZERO)
        );
    }
    for value in ["", "-1", "+1", "1.5", "invalid"] {
        assert_eq!(RetryAfter::from_header(value), None, "{value}");
    }
}
