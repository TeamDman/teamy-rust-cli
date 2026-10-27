use teamy_cancellation::CancellationError;
use teamy_cancellation::CancellationToken;
use teamy_rust_cli::cli::Command;
use teamy_rust_cli::cli::home::HomeArgs;
use teamy_rust_cli::cli::home::HomeCommand;
use teamy_rust_cli::cli::home::show::HomeShowArgs;

#[tokio::test]
async fn cancelled_dispatch_returns_typed_reason_before_command_runs() {
    let command = Command::Home(HomeArgs {
        command: HomeCommand::Show(HomeShowArgs),
    });
    let token = CancellationToken::new();
    token.request_cancel("dispatch cancelled");
    let error = command.invoke(token).await.expect_err("cancelled dispatch");
    assert!(error.downcast_ref::<CancellationError>().is_some());
    assert_eq!(error.to_string(), "dispatch cancelled");
}
