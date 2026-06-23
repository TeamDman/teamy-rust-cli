use super::CancellationState;
use super::CancellationToken;
use std::time::Duration;
use std::time::Instant;

#[test]
fn first_ctrl_c_requests_graceful_shutdown() {
    let mut state = CancellationState::new();

    let action = state.record_ctrl_c(Instant::now());

    assert_eq!(
        action,
        super::cancellation_state::CtrlCAction::RequestGracefulShutdown
    );
    assert!(state.is_cancelled());
}

#[test]
fn second_fast_ctrl_c_forces_exit() {
    let mut state = CancellationState::new();
    let now = Instant::now();
    state.record_ctrl_c(now);

    let action = state.record_ctrl_c(now + Duration::from_millis(250));

    assert_eq!(action, super::cancellation_state::CtrlCAction::ForceExit);
}

#[test]
fn second_slow_ctrl_c_stays_graceful() {
    let mut state = CancellationState::new();
    let now = Instant::now();
    state.record_ctrl_c(now);

    let action = state.record_ctrl_c(now + Duration::from_secs(2));

    assert_eq!(
        action,
        super::cancellation_state::CtrlCAction::RequestGracefulShutdown
    );
}

#[test]
fn cancellation_token_clones_share_cancellation() {
    let token = CancellationToken::new();
    let clone = token.clone();

    clone.request_cancel("profile stop");

    assert!(token.is_cancelled());
    assert_eq!(token.cancellation_reason().as_deref(), Some("profile stop"));
}

#[test]
fn cancellation_token_keeps_first_reason() {
    let token = CancellationToken::new();

    token.request_cancel("first");
    token.request_cancel("second");

    assert_eq!(token.cancellation_reason().as_deref(), Some("first"));
    assert!(
        token
            .bail_if_cancelled()
            .unwrap_err()
            .to_string()
            .contains("first")
    );
}
