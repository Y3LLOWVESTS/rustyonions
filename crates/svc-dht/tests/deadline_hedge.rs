use std::time::Duration;
use svc_dht::pipeline::hedging::race_hedged;
use tokio::time::sleep;

#[tokio::test]
async fn hedger_respects_budget_and_stagger() {
    // NOTE: We intentionally set budget to 20ms and stagger to 5ms, then make the
    // primary slow and hedges fast. Expect total elapsed < ~25ms and Ok(()).

    let budget = Duration::from_millis(20);
    let stagger = Duration::from_millis(5);

    let started = std::time::Instant::now();

    let out = race_hedged::<_, _, (), ()>(2, stagger, budget, |leg_idx| async move {
        if leg_idx == 0 {
            sleep(Duration::from_millis(100)).await; // slow primary hits timeout → hedges win
        } else {
            sleep(Duration::from_millis(1)).await; // fast hedge
        }
        Ok(())
    })
    .await;

    let elapsed = started.elapsed();
    assert!(out.is_ok(), "hedged race should succeed via hedge");
    assert!(elapsed < Duration::from_millis(30), "elapsed too large: {elapsed:?}");
}
