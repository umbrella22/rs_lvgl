#![cfg(feature = "embedscope")]

use escope_host::{protocol as p, Error, Session};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Application(Child);
impl Application {
    fn start(case: &str) -> (Self, String) {
        let mut child = Self(
            Command::new(env!("CARGO_BIN_EXE_rs-lvgl-embedscope-example"))
                .args([
                    "--listen",
                    "127.0.0.1:0",
                    "--exit-on-stdin-close",
                    "--brightness-case",
                    case,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let stdout = child.0.stdout.take().unwrap();
        let (ready, started) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = ready.send(result);
        });
        let line = started
            .recv_timeout(Duration::from_secs(2))
            .expect("native startup deadline")
            .unwrap();
        let endpoint = line
            .trim()
            .strip_prefix("LISTEN ")
            .expect("native listener startup");
        (child, endpoint.to_owned())
    }

    async fn finish(mut self) {
        // Finish the protocol assertions before requesting native teardown.
        assert!(
            self.0.try_wait().unwrap().is_none(),
            "application exited early"
        );
        drop(self.0.stdin.take().expect("application stdin pipe"));
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "native teardown failed: {status}");
                return;
            }
            assert!(Instant::now() < deadline, "application did not shut down");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("child status at test failure: {:?}", self.0.try_wait());
        }
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

fn point(node: &escope_host::UiNode, numerator: u32, denominator: u32) -> (u32, u32) {
    assert!(node.width > 0 && node.height > 0 && numerator < denominator);
    (
        u32::try_from(node.x).unwrap() + node.width * numerator / denominator,
        u32::try_from(node.y).unwrap() + node.height / 2,
    )
}

fn pixel(capture: &escope_host::Capture, (x, y): (u32, u32)) -> [u8; 3] {
    assert!(x < capture.meta.width && y < capture.meta.height);
    let start = ((y * capture.meta.width + x) * 3) as usize;
    capture.rgb[start..start + 3].try_into().unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn real_native_pixels_input_fixed_capture_and_reconnect() {
    let (child, endpoint) = Application::start("normal");
    bad_frame_then_fragmented_hello(&endpoint);
    let session = connect(&endpoint).await;
    let info = session.info().await.unwrap();
    assert_eq!(info.model, "LVGL 9.6 / memory display");
    assert_ne!(info.boot_id, 0);
    assert!(!session.supports(p::capability::CRASH));
    assert!(session.metrics().await.unwrap().heap_free.is_none());
    let tree = session.ui_tree().await.unwrap();
    assert_eq!(tree.len(), 8);
    let button = tree
        .iter()
        .find(|n| n.kind == "button" && n.name == "settings.increment")
        .expect("real button geometry");
    assert_eq!((button.width, button.height), (224, 56));
    let (x, y) = point(button, 1, 2);
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
    let reconnected = connect(&endpoint).await;
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
    child.finish().await;
}

#[tokio::test(flavor = "current_thread")]
async fn brightness_evidence_distinguishes_input_state_and_completed_pixels() {
    for (case, numerator, denominator, result) in [
        ("normal", 7, 8, "rejected"),
        ("normal", 1, 2, "presented"),
        ("ignore-input", 1, 2, "input-unobserved"),
        ("hold-preview", 1, 2, "preview-unconfirmed"),
    ] {
        let (application, endpoint) = Application::start(case);
        let session = connect(&endpoint).await;
        let tree = session.ui_tree().await.unwrap();
        let slider = tree
            .iter()
            .find(|node| node.kind == "slider" && node.name == "action:settings.brightness")
            .expect("native brightness slider");
        let preview = tree
            .iter()
            .find(|node| node.name == "settings.preview")
            .expect("native preview swatch");
        assert_eq!((slider.width, slider.height), (224, 40));
        let (x, y) = point(slider, numerator, denominator);
        let sample = point(preview, 1, 2);
        let mut logs = session.subscribe();
        session.start_logs(3).await.unwrap();
        let before = session.capture().await.unwrap();
        let initial_rgb = [32, 114, 96];
        assert_eq!(pixel(&before, sample), initial_rgb);

        if result == "presented" {
            session.tap(x, y, 1000).await.unwrap();
            session
                .request_raw(
                    p::Operation::Cancel.into(),
                    &encode(&p::Cancel { request_id: 1 }),
                )
                .await
                .unwrap();
            let cancelled = session.capture().await.unwrap();
            assert_eq!(
                pixel(&cancelled, sample),
                initial_rgb,
                "Cancel must not commit the slider edit"
            );
            assert!(
                logs.try_recv().is_err(),
                "a cancelled tap must not emit result evidence"
            );
        }

        // A successful transport reply only admits the input. The application
        // log and the sampled pixel are independent acceptance oracles.
        session.tap(x, y, 60).await.unwrap();
        let log = tokio::time::timeout(Duration::from_secs(1), logs.recv())
            .await
            .expect("outcome deadline")
            .unwrap()
            .log()
            .unwrap();
        assert_eq!(log.session, session.id());
        assert!(!log.truncated);
        assert_eq!(log.target, "escope.ui.evidence.v1");
        let evidence: serde_json::Value = serde_json::from_str(&log.message).unwrap();
        let frame = evidence["frame"].clone();
        assert_eq!(
            evidence,
            serde_json::json!({
                "control": "settings.brightness", "result": result, "frame": frame,
            })
        );

        let after = session.capture().await.unwrap();
        if result == "presented" {
            let frame = evidence["frame"].as_u64().unwrap();
            assert!(frame > before.meta.frame_id && after.meta.frame_id >= frame);
            // The midpoint of the 0..100 range is 50. The visible swatch must
            // contain the resulting RGB, not merely some changed screen bytes.
            assert_eq!(pixel(&after, sample), [32, 174, 96]);
            // Disconnect with an outstanding observation. It releases the
            // pointer and does not export evidence to the next session.
            let (left, y) = point(slider, 1, 4);
            session.tap(left, y, 1000).await.unwrap();
            let previous_session = session.id();
            session.close();
            let reconnected = connect(&endpoint).await;
            assert_ne!(reconnected.id(), previous_session);
            let mut later_logs = reconnected.subscribe();
            reconnected.start_logs(3).await.unwrap();
            let released = reconnected.ui_tree().await.unwrap();
            assert!(released
                .iter()
                .all(|n| n.flags & rs_lvgl::sys::lv_state_t_LV_STATE_PRESSED == 0));
            // The prior outcome and any abandoned observation are absent.
            let evidence =
                tokio::time::timeout(Duration::from_millis(300), later_logs.recv()).await;
            assert!(evidence.is_err());
            reconnected.close();
        } else {
            assert!(evidence["frame"].is_null());
            assert_eq!(pixel(&after, sample), initial_rgb);
            session.close();
        }
        application.finish().await;
    }
}
