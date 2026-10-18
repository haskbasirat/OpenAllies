//! Local integration example only: a caller-selected file crosses an ephemeral
//! loopback TCP connection and is verified into a create-new destination.

use aa_transfer::{receive_file_new, send_file};
use std::error::Error;
use std::io;
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: loopback_transfer <existing-source-file> <new-destination-file>",
        )
        .into());
    }
    let source = PathBuf::from(&arguments[0]);
    let destination = PathBuf::from(&arguments[1]);

    // No wildcard bind, configurable listening address, or public listener.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let mut sending_socket = TcpStream::connect(listener.local_addr()?)?;
    let receiver = thread::spawn(move || {
        let (mut socket, _) = listener.accept()?;
        receive_file_new(&mut socket, destination)
            .map_err(|error| io::Error::other(error.to_string()))
    });

    let sent = send_file(source, &mut sending_socket);
    // EOF also reaches the receiver after a failed/truncated send. The transfer
    // itself has no retry or acknowledgement; the test joins both local endpoints.
    let _ = sending_socket.shutdown(Shutdown::Write);
    drop(sending_socket);
    let received = receiver
        .join()
        .map_err(|_| io::Error::other("receiver thread panicked"))??;
    let sent = sent?;
    if sent != received {
        return Err(io::Error::other("endpoint receipts differ").into());
    }
    let digest = received
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!("Verified {} bytes; SHA-256 {digest}", received.length);
    Ok(())
}
