use tokio::sync::oneshot::Sender;
use tokio::signal;
use tokio::signal::unix;
use tokio::signal::unix::SignalKind;

pub async fn wait_for_shutdown_signals(shutdown_tx: Sender<()>) {
    println!("Waiting for shutdown signals...");

    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to listen for SIGINT");
        println!("Received SIGINT");
    };

    let sigterm = async {
        let mut sigterm = unix::signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");
        sigterm.recv().await;
        println!("Received SIGTERM");
    };

    // Wait for either SIGINT or SIGTERM
    tokio::select! {
        _ = ctrl_c => { println!("Ctrl+C pressed (SIGINT)"); },
        _ = sigterm => { println!("Terminate signal received (SIGTERM)"); },
    }

    println!("Sending shutdown signal...");
    // Send the shutdown signal
    let _ = shutdown_tx.send(());
}
