use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread::{sleep, JoinHandle};
use std::{io, thread};

pub fn start_server(
    bind_address: SocketAddr,
    is_running: Arc<AtomicBool>,
    receive_tx: Sender<(SocketAddr, Vec<u8>)>,
    send_rx: Receiver<(SocketAddr, Vec<u8>)>,
) -> Result<(JoinHandle<()>, JoinHandle<()>), String> {
    let receive_socket = UdpSocket::bind(&bind_address).map_err(|error| {
        format!(
            "Failed to bind to address {}: {}. Is the port already in use?",
            bind_address, error
        )
    })?;

    let send_socket = receive_socket
        .try_clone()
        .map_err(|error| format!("Failed to clone socket: {}", error))?;

    receive_socket
        .set_nonblocking(true)
        .map_err(|error| format!("Failed to set non-blocking: {}", error))?;

    let is_running_clone = is_running.clone();
    let receive_thread = thread::spawn(move || {
        let mut buffer = [0; 1024];

        loop {
            let result = receive_socket.recv_from(&mut buffer);

            match result {
                Ok((amt, src)) => {
                    if let Err(error) = receive_tx.send((src, buffer[..amt].to_vec())) {
                        // TODO Don't panic here
                        panic!("Failed to send packet to receive channel: {}", error);
                    }
                }
                Err(error) => {
                    if error.kind() != io::ErrorKind::WouldBlock {
                        // TODO Don't panic here
                        panic!("Failed to receive packet: {}", error);
                    }

                    if !is_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    sleep(std::time::Duration::from_millis(50));
                    continue;
                }
            };
        }
    });

    let is_running_clone = is_running.clone();
    let send_thread = thread::spawn(move || loop {
        let outgoing_message = send_rx.try_recv();

        match outgoing_message {
            Ok((socket_addr, message)) => {
                send_socket.send_to(&message, socket_addr).unwrap();
            }
            Err(error) => {
                if error != mpsc::TryRecvError::Empty {
                    panic!("Failed to receive outgoing message: {}", error);
                }
            }
        }

        if !is_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        sleep(std::time::Duration::from_millis(50));
    });

    Ok((receive_thread, send_thread))
}
