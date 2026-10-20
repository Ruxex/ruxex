use std::future::Future;

const TEST_THREAD_STACK_SIZE: usize = if cfg!(target_os = "macos") {
    // Rama 0.3's macOS proxy and lifecycle service stack is substantially deeper than
    // the Linux path when exercised through the real listener.
    64 * 1024 * 1024
} else {
    16 * 1024 * 1024
};

pub(crate) fn run_on_large_stack<F, T>(future: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .name("network-proxy-test".to_string())
        .stack_size(TEST_THREAD_STACK_SIZE)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build test runtime")
                .block_on(future)
        })
        .expect("spawn large-stack test thread")
        .join()
        .expect("large-stack test thread panicked")
}
