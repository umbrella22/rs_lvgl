#![cfg(feature = "embedscope")]

use escope_host::{protocol as p, Error, Session};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Application(Child);
impl Drop for Application {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn encode<T: p::minicbor::Encode<()>>(value: &T) -> Vec<u8> {
    let mut bytes = [0; p::MAX_PAYLOAD];
    let len = p::encode_payload(value, &mut bytes).unwrap();
    bytes[..len].to_vec()
}

async fn connect(endpoint: &str) -> Session {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match Session::connect(endpoint, 115200, Duration::from_millis(500)).await {
            Ok(session) => return session,
            Err(error) => {
                assert!(Instant::now() < deadline, "connect failed: {error}");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

fn bad_frame_then_fragmented_hello(endpoint: &str) {
    let mut stream = TcpStream::connect(endpoint.trim_start_matches("tcp://")).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(&[3, 1, 2, 0]).unwrap();
    let request = p::Envelope {
        version: p::VERSION,
        kind: p::Kind::Request,
        session: 0,
        id: 1,
        op: p::Operation::Hello.into(),
        status: p::Status::Ok,
        payload: &[],
    };
    let mut frame = [0; p::MAX_FRAME];
    let n = p::encode_frame(&request, &mut frame).unwrap();
    for fragment in frame[..n].chunks(3) {
        stream.write_all(fragment).unwrap();
    }
    let mut decoder = p::FrameDecoder::new();
    let mut scratch = [0; p::MAX_FRAME];
    let mut bytes = [0; 256];
    loop {
        let n = stream.read(&mut bytes).unwrap();
        assert!(n > 0);
        for byte in &bytes[..n] {
            if let Some(Ok(frame)) = decoder.push(*byte) {
                let reply = p::decode_frame(frame, &mut scratch).unwrap();
                assert_eq!(reply.status, p::Status::Ok);
                assert_ne!(reply.session, 0);
                return;
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn real_native_pixels_input_fixed_capture_and_reconnect() {
    let mut child = Application(
        Command::new(env!("CARGO_BIN_EXE_rs-lvgl-embedscope-example"))
            .args(["--listen", "127.0.0.1:0", "--run-for-ms", "5500"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(child.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let endpoint = line
        .trim()
        .strip_prefix("LISTEN ")
        .expect("native listener startup");
    bad_frame_then_fragmented_hello(endpoint);
    let session = connect(endpoint).await;
    let info = session.info().await.unwrap();
    assert_eq!(info.model, "LVGL 9.6 / memory display");
    assert_ne!(info.boot_id, 0);
    assert!(!session.supports(p::capability::CRASH));
    assert!(session.metrics().await.unwrap().heap_free.is_none());
    let tree = session.ui_tree().await.unwrap();
    assert!(tree.len() >= 5);
    let button = tree
        .iter()
        .find(|n| n.width == 224 && n.height == 56)
        .expect("real button geometry");
    let (x, y) = ((button.x + 112) as u32, (button.y + 28) as u32);
    let revision = tree[0].revision;
    let mut events = session.subscribe();
    session.start_logs(3).await.unwrap();
    let before = session.capture().await.unwrap();
    assert_eq!((before.meta.width, before.meta.height), (320, 240));
    assert!(before.rgb.windows(3).any(|p| p != &before.rgb[..3]));

    let bytes = session
        .request_raw(p::Operation::CaptureStart.into(), &[])
        .await
        .unwrap();
    let held: p::CaptureMeta = p::decode_payload(&bytes).unwrap();
    assert!(matches!(
        session
            .request_raw(p::Operation::CaptureStart.into(), &[])
            .await,
        Err(Error::Device(p::Status::Busy))
    ));
    let query = encode(&p::CaptureRead {
        capture_id: held.capture_id,
        offset: 0,
        length: 768,
    });
    let fixed_before = session
        .request_raw(p::Operation::CaptureRead.into(), &query)
        .await
        .unwrap();
    session.tap(x, y, 60).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let log = tokio::time::timeout(Duration::from_secs(1), events.recv())
        .await
        .unwrap()
        .unwrap()
        .log()
        .unwrap();
    assert_eq!(log.target, "example.counter");
    assert_eq!(log.message, "counter=1");
    let fixed_after = session
        .request_raw(p::Operation::CaptureRead.into(), &query)
        .await
        .unwrap();
    assert_eq!(
        fixed_before, fixed_after,
        "capture bytes must stay fixed across rendering"
    );
    session
        .request_raw(
            p::Operation::CaptureEnd.into(),
            &encode(&p::CaptureEnd {
                capture_id: held.capture_id,
            }),
        )
        .await
        .unwrap();
    let after = session.capture().await.unwrap();
    assert_ne!(
        before.rgb, after.rgb,
        "native input must change rendered pixels"
    );
    assert!(after.meta.frame_id > before.meta.frame_id);
    assert!(matches!(
        session
            .request_raw(
                p::Operation::UiTree.into(),
                &encode(&p::TreeQuery {
                    cursor: 0,
                    revision
                })
            )
            .await,
        Err(Error::Device(p::Status::Stale))
    ));
    assert!(matches!(
        session.tap(320, 0, 60).await,
        Err(Error::Device(p::Status::InvalidArgument))
    ));
    assert!(matches!(
        session.request_raw(65535, &[]).await,
        Err(Error::Device(p::Status::Unsupported))
    ));

    let bytes = session
        .request_raw(p::Operation::CaptureStart.into(), &[])
        .await
        .unwrap();
    let abandoned: p::CaptureMeta = p::decode_payload(&bytes).unwrap();
    session.tap(x, y, 1000).await.unwrap();
    let old_session = session.id();
    session.close();
    let reconnected = connect(endpoint).await;
    assert_ne!(old_session, reconnected.id());
    assert!(matches!(
        reconnected
            .request_raw(
                p::Operation::CaptureRead.into(),
                &encode(&p::CaptureRead {
                    capture_id: abandoned.capture_id,
                    offset: 0,
                    length: 1
                })
            )
            .await,
        Err(Error::Device(p::Status::Stale))
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    reconnected.capture().await.unwrap();
    let released = reconnected.ui_tree().await.unwrap();
    assert!(released
        .iter()
        .all(|n| n.flags & rs_lvgl::sys::lv_state_t_LV_STATE_PRESSED == 0));
    let mut events = reconnected.subscribe();
    reconnected.start_logs(3).await.unwrap();
    reconnected.tap(x, y, 60).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let log = tokio::time::timeout(Duration::from_secs(1), events.recv())
        .await
        .unwrap()
        .unwrap()
        .log()
        .unwrap();
    assert_eq!(
        log.message, "counter=3",
        "disconnect completes exactly one accepted tap"
    );
    assert!(reconnected.ui_tree().await.unwrap()[0].revision > revision);
    reconnected.tap(x, y, 1000).await.unwrap();
    reconnected
        .request_raw(
            p::Operation::Cancel.into(),
            &encode(&p::Cancel { request_id: 1 }),
        )
        .await
        .unwrap();
    reconnected.tap(x, y, 60).await.unwrap();
    let log = tokio::time::timeout(Duration::from_secs(1), events.recv())
        .await
        .unwrap()
        .unwrap()
        .log()
        .unwrap();
    assert_eq!(
        log.message, "counter=4",
        "explicit cancellation must not click"
    );
    reconnected.close();

    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "native teardown failed: {status}");
            break;
        }
        assert!(Instant::now() < deadline, "application did not shut down");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
