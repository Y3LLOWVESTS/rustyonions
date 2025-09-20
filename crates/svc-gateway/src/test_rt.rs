#[macro_export]
macro_rules! test_both_runtimes {
    ($name:ident, $body:block) => {
        #[cfg(feature = "rt-multi-thread")]
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() $body

        #[cfg(feature = "rt-current-thread")]
        #[tokio::test(flavor = "current_thread")]
        async fn $name() $body
    };
}
