use super::*;
use std::net::TcpListener;
use std::thread;

fn server(responses: Vec<(Vec<u8>, u16)>) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        for pack in [PACK1, PACK2, PACK3] {
            let mut request = vec![0; pack.len()];
            stream.read_exact(&mut request).unwrap();
            assert_eq!(request, pack);
            stream.write_all(&[0; 16]).unwrap();
        }
        for (payload, inflate_size) in responses {
            let mut request = [0; 1];
            stream.read_exact(&mut request).unwrap();
            let mut header = [0u8; 16];
            header[12..14].copy_from_slice(&(payload.len() as u16).to_le_bytes());
            header[14..16].copy_from_slice(&inflate_size.to_le_bytes());
            stream.write_all(&header).unwrap();
            stream.write_all(&payload).unwrap();
        }
    });
    (addr, handle)
}

#[test]
fn corrupt_compression_returns_error_and_connection_can_rotate() {
    let (primary, primary_thread) = server(vec![(vec![1, 2, 3], 12)]);
    let (backup, backup_thread) = server(vec![(vec![42], 1)]);
    let config = TcpConfig {
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&config, &[primary, backup]).unwrap();
    let error = send_recv_decompress(&mut tcp, &[1], "corrupt").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    tcp.reconnect().unwrap();
    assert_eq!(tcp.config().ip, Some(backup));
    assert_eq!(send_recv_decompress(&mut tcp, &[1], "valid").unwrap(), [42]);
    primary_thread.join().unwrap();
    backup_thread.join().unwrap();
}

#[test]
fn compressed_payload_length_and_allocation_are_checked() {
    let payload = miniz_oxide::deflate::compress_to_vec_zlib(&[7; 100], 6);
    let len = payload.len() as u16;
    assert_eq!(
        decode_response(payload.clone(), len, 100).unwrap(),
        [7; 100]
    );
    for declared in [50, 101] {
        assert_eq!(
            decode_response(payload.clone(), len, declared)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }
    assert_eq!(
        decode_response(vec![1], 2, 2).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn pool_skips_unavailable_server_at_initial_connect() {
    let unavailable = TcpListener::bind("127.0.0.1:0").unwrap();
    let dead_addr = unavailable.local_addr().unwrap();
    drop(unavailable);
    let (backup, handle) = server(vec![]);
    let tcp = Tcp::with_servers(&TcpConfig::default(), &[dead_addr, backup]).unwrap();
    assert_eq!(tcp.config().ip, Some(backup));
    handle.join().unwrap();
}

#[test]
fn empty_pool_is_invalid_input() {
    assert_eq!(
        Tcp::with_servers(&TcpConfig::default(), &[])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn failed_request_automatically_rotates_and_retries_on_backup() {
    let (primary, primary_thread) = server(vec![]); // closes after handshake
    let (backup, backup_thread) = server(vec![(vec![9], 1)]);
    let cfg = TcpConfig {
        auto_reconnect: 2,
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&cfg, &[primary, backup]).unwrap();
    primary_thread.join().unwrap();
    assert_eq!(send_recv_decompress(&mut tcp, &[1], "retry").unwrap(), [9]);
    assert_eq!(tcp.config().ip, Some(backup));
    backup_thread.join().unwrap();
}

#[test]
fn pinned_config_reconnects_to_the_same_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            for pack in [PACK1, PACK2, PACK3] {
                let mut request = vec![0; pack.len()];
                stream.read_exact(&mut request).unwrap();
                stream.write_all(&[0; 16]).unwrap();
            }
        }
    });
    let config = TcpConfig {
        ip: Some(addr),
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_config(&config).unwrap();
    tcp.reconnect().unwrap();
    assert_eq!(tcp.config().ip, Some(addr));
    assert!(tcp.servers.is_empty());
    handle.join().unwrap();
}

#[test]
fn exhausted_pool_returns_error_without_discarding_candidates() {
    let (addr, handle) = server(vec![]);
    let cfg = TcpConfig {
        timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&cfg, &[addr]).unwrap();
    handle.join().unwrap();
    assert!(tcp.reconnect().is_err());
    assert_eq!(tcp.servers, vec![addr]);
}
