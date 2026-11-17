use super::*;
use std::net::TcpListener;
use std::thread;

fn server(responses: Vec<(Vec<u8>, u16)>) -> (SocketAddr, thread::JoinHandle<()>) {
    server_with_lengths(
        responses
            .into_iter()
            .map(|(body, size)| {
                let wire = body.len() as u16;
                (body, size, wire)
            })
            .collect(),
        1,
    )
}

fn server_with_lengths(
    responses: Vec<(Vec<u8>, u16, u16)>,
    request_len: usize,
) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        for &pack in HANDSHAKE {
            let mut request = vec![0; pack.len()];
            stream.read_exact(&mut request).unwrap();
            assert_eq!(request, pack);
            stream.write_all(&[0; 16]).unwrap();
        }
        for (payload, inflate_size, wire_size) in responses {
            let mut request = vec![0; request_len];
            stream.read_exact(&mut request).unwrap();
            let mut header = [0u8; 16];
            header[12..14].copy_from_slice(&wire_size.to_le_bytes());
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
        auto_reconnect: 0, // 关闭默认重试：本用例要观察损坏响应本身的错误
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
            for &pack in HANDSHAKE {
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

#[test]
fn corrupt_compression_automatically_retries_the_whole_request() {
    let (primary, primary_thread) = server(vec![(vec![1, 2, 3], 12)]);
    let (backup, backup_thread) = server(vec![(vec![42], 1)]);
    let cfg = TcpConfig {
        auto_reconnect: 2,
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&cfg, &[primary, backup]).unwrap();
    assert_eq!(
        send_recv_decompress(&mut tcp, &[1], "corrupt retry").unwrap(),
        [42]
    );
    primary_thread.join().unwrap();
    backup_thread.join().unwrap();
}

#[test]
fn partial_body_automatically_reconnects_before_retry() {
    let (primary, first) = server_with_lengths(vec![(vec![1], 2, 2)], 1);
    let (backup, second) = server(vec![(vec![42], 1)]);
    let cfg = TcpConfig {
        auto_reconnect: 2,
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&cfg, &[primary, backup]).unwrap();
    assert_eq!(
        send_recv_decompress(&mut tcp, &[1], "partial body").unwrap(),
        [42]
    );
    first.join().unwrap();
    second.join().unwrap();
}

#[test]
fn malformed_quote_is_retried_instead_of_returning_partial_data() {
    use crate::tcp::stock::SecurityQuotes;
    let hex = "0114010000303030303031fe08a61242440a45bb9fe30ae612a9853d0d31160e4e91981e98ed1e00bfaf0700019f12074102a724ab3b4203911295164304a8288a234405a33296457501000000000000fe08";
    let body: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let size = body.len() as u16;
    let mut parser = SecurityQuotes::default();
    let request_len = parser.send().len();
    let (primary, first) = server_with_lengths(vec![(vec![0, 0, 1, 0], 4, 4)], request_len);
    let (backup, second) = server_with_lengths(vec![(body, size, size)], request_len);
    let cfg = TcpConfig {
        auto_reconnect: 2,
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let mut tcp = Tcp::with_servers(&cfg, &[primary, backup]).unwrap();
    let quotes = parser.recv_parsed(&mut tcp).unwrap();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].price, 11.9);
    assert_eq!(tcp.config().auto_reconnect, 2);
    first.join().unwrap();
    second.join().unwrap();
}

#[test]
fn failed_handshake_does_not_recursively_reconnect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_millis(300);
        let mut count = 0;
        while std::time::Instant::now() < deadline && count < 3 {
            match listener.accept() {
                Ok((stream, _)) => {
                    drop(stream);
                    count += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(1))
                }
                Err(e) => panic!("{e}"),
            }
        }
        count
    });
    let cfg = TcpConfig {
        auto_reconnect: 2,
        ip: Some(addr),
        timeout: Duration::from_millis(100),
        ..Default::default()
    };
    assert!(Tcp::with_config(&cfg).is_err());
    assert_eq!(handle.join().unwrap(), 1);
}

#[test]
fn zero_retry_attempts_is_an_input_error() {
    let (addr, handle) = server(vec![]);
    let mut tcp = Tcp::with_config(&TcpConfig {
        ip: Some(addr),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        tcp.retry(|_| Ok(()), 0).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    handle.join().unwrap();
}

/// 回归：握手只发 PACK1 + PACK2。发送 PACK3 后服务端会掐断行情类权限
/// （K 线只回 `20 03`、行情/逐笔回 0 条），实测去掉后立即恢复。
#[test]
fn handshake_sends_only_first_two_packs() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut received = Vec::new();
        for &pack in HANDSHAKE {
            let mut request = vec![0; pack.len()];
            stream.read_exact(&mut request).unwrap();
            received.extend_from_slice(&request);
            stream.write_all(&[0; 16]).unwrap();
        }
        // 客户端握手完成后即断开：之后不应再有任何字节（尤其是 PACK3）
        stream.read_to_end(&mut received).unwrap();
        received
    });
    let config = TcpConfig {
        timeout: Duration::from_secs(1),
        ip: Some(addr),
        ..Default::default()
    };
    drop(Tcp::with_config(&config).unwrap());
    assert_eq!(handle.join().unwrap(), [PACK1, PACK2].concat());
}
