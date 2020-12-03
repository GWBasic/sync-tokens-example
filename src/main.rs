use std::io::{ Error, ErrorKind };

use async_std::io::Result;
use async_std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream, SocketAddr};
use async_std::task;
use async_std::task::JoinHandle;

use sync_tokens::cancelation_token::{ Cancelable, CancelationToken };
use sync_tokens::completion_token::{ Completable, CompletionToken };

// Starts running a server on a background task
pub fn run_server() -> (JoinHandle<Result<()>>, CompletionToken<Result<SocketAddr>>, CancelationToken) {
    // This CompletionToken allows the caller to wait until the server is actually listening
    // The caller gets completion_token, which it can await on
    // completable is used to signal to completion_token
    let (completion_token, completable) = CompletionToken::new();

    // This CancelationToken allows the caller to stop the server
    // The caller gets cancelation_token
    // cancelable is used to allow canceling a call to await
    let (cancelation_token, cancelable) = CancelationToken::new();

    // The server is started on a background task, and the future returned
    let server_future = task::spawn(run_server_int(completable, cancelable));

    (server_future, completion_token, cancelation_token)
}

async fn run_server_int(completable: Completable<Result<SocketAddr>>, cancelable: Cancelable) -> Result<()> {

    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let listener = TcpListener::bind(socket_addr).await?;

    // Inform that the server is listening
    let local_addr = listener.local_addr();
    completable.complete(local_addr);

    // Create a future that waits for an incoming socket
    let mut incoming_future = task::spawn(accept(listener));
    
    loop {
        // Wait for either the incoming socket (via incoming_future) or for the CancelationToken
        // to be canceled.
        // When the CancelationToken is canceled, the error is returned
        let (listener, _) = cancelable.allow_cancel(
            incoming_future, 
            Err(Error::new(ErrorKind::Interrupted, "Server terminated")))
            .await?;

        incoming_future = task::spawn(accept(listener));
    }
}

async fn accept(listener: TcpListener) -> Result<(TcpListener, TcpStream)> {
    let (stream, _) = listener.accept().await?;
    Ok((listener, stream))
}

#[async_std::main]
async fn main() {
    let (server_future, completion_token, cancelation_token) = run_server();

    println!("Server is starting");

    // Wait for the server to start
    let local_addr = completion_token.await.unwrap();

    println!("Server is listening at {}", local_addr);
    println!("Push Return to stop the server");

    let _ = std::io::stdin().read_line(&mut String::new()).unwrap();

    // Stop the server
    cancelation_token.cancel();

    // Wait for the server to shut down
    let err = server_future.await.unwrap_err();

    println!("Server ended: {}", err);
}
