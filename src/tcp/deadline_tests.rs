use super::*;
use std::net::TcpListener;
use std::thread;

fn listen(script: impl FnOnce(TcpStream) + Send + 'static) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        script(stream);
    });
    (addr, handle)
}

fn handshake(stream: &mut TcpStream) {
    for pack in [PACK1, PACK2, PACK3] {
        let mut input = vec![0; pack.len()];
        stream.read_exact(&mut input).unwrap();
        assert_eq!(input, pack);
        stream.write_all(&[0; 16]).unwrap();
    }
}

fn header(size: u16) -> [u8; 16] {
    let mut header = [0; 16];
    header[12..14].copy_from_slice(&size.to_le_bytes());
    header[14..16].copy_from_slice(&size.to_le_bytes());
    header
}

fn config() -> TcpConfig {
    TcpConfig {
        timeout: Duration::from_secs(5),
        auto_reconnect: 4,
        ..Default::default()
    }
}

#[test]
fn expired_deadline_never_opens_a_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let cfg = TcpConfig {
        ip: Some(listener.local_addr().unwrap()),
        ..config()
    };
    assert_eq!(
        Tcp::with_config_until(&cfg, Instant::now())
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::TimedOut
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn handshake_and_server_rotation_share_one_deadline() {
    let (addr, handle) = listen(|mut stream| {
        let mut input = vec![0; PACK1.len()];
        stream.read_exact(&mut input).unwrap();
        thread::sleep(Duration::from_millis(400));
    });
    let backup = TcpListener::bind("127.0.0.1:0").unwrap();
    backup.set_nonblocking(true).unwrap();
    let started = Instant::now();
    let result = Tcp::with_servers_until(
        &config(),
        &[addr, backup.local_addr().unwrap()],
        started + Duration::from_millis(80),
    );
    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "handshake exceeded total budget"
    );
    assert_eq!(
        backup.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    handle.join().unwrap();
}

#[test]
fn slow_drip_cannot_extend_header_or_body_deadline() {
    for body in [false, true] {
        let (addr, handle) = listen(move |mut stream| {
            handshake(&mut stream);
            let mut request = [0];
            stream.read_exact(&mut request).unwrap();
            let bytes = if body {
                stream.write_all(&header(64)).unwrap();
                vec![42; 64]
            } else {
                header(0).to_vec()
            };
            for byte in bytes {
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let mut tcp = Tcp::with_config(&TcpConfig {
            ip: Some(addr),
            ..config()
        })
        .unwrap();
        let started = Instant::now();
        tcp.set_deadline(Some(started + Duration::from_millis(80)));
        assert!(send_recv_decompress(&mut tcp, &[1], "slow drip").is_err());
        assert!(started.elapsed() < Duration::from_millis(300));
        assert!(tcp.needs_reconnect);
        handle.join().unwrap();
    }
}

#[test]
fn retry_backoff_does_not_outlive_deadline() {
    let (addr, handle) = listen(|mut stream| {
        handshake(&mut stream);
    });
    let mut cfg = config();
    cfg.retry_delay_ms = 5_000;
    let mut tcp = Tcp::with_servers(&cfg, &[addr]).unwrap();
    handle.join().unwrap();
    let started = Instant::now();
    tcp.set_deadline(Some(started + Duration::from_millis(80)));
    assert_eq!(
        send_recv_decompress(&mut tcp, &[1], "retry wait")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::TimedOut
    );
    assert!(started.elapsed() < Duration::from_millis(300));
}

#[test]
fn empty_quote_recheck_sleep_obeys_deadline() {
    use crate::tcp::stock::Client;
    let (addr, handle) = listen(|mut stream| {
        handshake(&mut stream);
        let mut request = [0; 29];
        stream.read_exact(&mut request).unwrap();
        stream.write_all(&header(4)).unwrap();
        stream.write_all(&[0; 4]).unwrap();
        let mut next = [0];
        let _ = stream.read(&mut next);
    });
    let cfg = TcpConfig {
        recheck_empty: true,
        ..config()
    };
    let mut client = Client::with_servers(&cfg, &[addr]).unwrap();
    let started = Instant::now();
    client
        .tcp
        .set_deadline(Some(started + Duration::from_millis(80)));
    assert_eq!(
        client.quotes(&[(0, "000001")]).unwrap_err().kind(),
        std::io::ErrorKind::TimedOut
    );
    assert!(started.elapsed() < Duration::from_millis(250));
    handle.join().unwrap();
}

#[test]
fn fresh_deadline_reconnects_after_interrupted_frame() {
    let (primary, first) = listen(|mut stream| {
        handshake(&mut stream);
        let mut request = [0];
        stream.read_exact(&mut request).unwrap();
        stream.write_all(&header(2)).unwrap();
        stream.write_all(&[9]).unwrap();
        let mut next = [0];
        let _ = stream.read(&mut next);
    });
    let (backup, second) = listen(|mut stream| {
        handshake(&mut stream);
        let mut request = [0];
        stream.read_exact(&mut request).unwrap();
        stream.write_all(&header(1)).unwrap();
        stream.write_all(&[42]).unwrap();
    });
    let mut tcp = Tcp::with_servers(&config(), &[primary, backup]).unwrap();
    tcp.set_deadline(Some(Instant::now() + Duration::from_millis(80)));
    assert!(send_recv_decompress(&mut tcp, &[1], "partial frame").is_err());
    assert!(tcp.needs_reconnect);
    tcp.set_deadline(Some(Instant::now() + Duration::from_secs(1)));
    assert_eq!(
        send_recv_decompress(&mut tcp, &[1], "new round").unwrap(),
        [42]
    );
    assert_eq!(tcp.config().ip, Some(backup));
    first.join().unwrap();
    second.join().unwrap();
}

#[test]
fn blocked_write_obeys_deadline() {
    let (addr, handle) = listen(|mut stream| {
        handshake(&mut stream);
        thread::sleep(Duration::from_millis(400));
    });
    let mut tcp = Tcp::with_config(&TcpConfig {
        ip: Some(addr),
        ..config()
    })
    .unwrap();
    let request = vec![1; 16 * 1024 * 1024];
    let started = Instant::now();
    tcp.set_deadline(Some(started + Duration::from_millis(80)));
    assert!(tcp.send_recv(&request).is_err());
    assert!(started.elapsed() < Duration::from_millis(300));
    assert!(tcp.needs_reconnect);
    handle.join().unwrap();
}
