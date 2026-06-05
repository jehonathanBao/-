use btc_toxic_flow_monitor_rs::api::discord_notification_routes::{
    reserve_discord_push_for_tests, reset_discord_push_limits_for_tests,
};

#[test]
fn duplicate_discord_push_is_suppressed_for_same_signal_key() {
    reset_discord_push_limits_for_tests();

    assert_eq!(reserve_discord_push_for_tests("sig_001"), None);
    assert_eq!(
        reserve_discord_push_for_tests("sig_001"),
        Some("DUPLICATE_PUSH_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
}

#[test]
fn burst_discord_pushes_are_rate_limited() {
    reset_discord_push_limits_for_tests();

    for index in 0..5 {
        assert_eq!(
            reserve_discord_push_for_tests(&format!("sig_{index}")),
            None
        );
    }
    assert_eq!(
        reserve_discord_push_for_tests("sig_over_limit"),
        Some("RATE_LIMITED")
    );

    reset_discord_push_limits_for_tests();
}
