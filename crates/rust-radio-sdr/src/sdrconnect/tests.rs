use super::*;
use std::net::TcpListener;
use std::thread::{self, JoinHandle};

fn fixture(run: impl FnOnce(WebSocket<TcpStream>) + Send + 'static) -> (Endpoint, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = Endpoint(listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        run(tungstenite::accept(stream).unwrap());
    });
    (endpoint, handle)
}
fn request(peer: &mut WebSocket<TcpStream>) -> Envelope {
    match peer.read().unwrap() {
        Message::Text(json) => serde_json::from_str(&json).unwrap(),
        _ => panic!("expected JSON request"),
    }
}
fn response(peer: &mut WebSocket<TcpStream>, property: &str, value: &str) {
    let envelope = Envelope {
        event_type: "get_property_response".into(),
        property: property.into(),
        value: value.into(),
        device: None,
    };
    peer.send(Message::Text(
        serde_json::to_string(&envelope).unwrap().into(),
    ))
    .unwrap();
}
fn packet(tag: u16, samples: &[i16]) -> Vec<u8> {
    let mut packet = tag.to_le_bytes().to_vec();
    for sample in samples {
        packet.extend_from_slice(&sample.to_le_bytes());
    }
    packet
}

#[test]
fn endpoint_requires_explicit_local_unicast_address_without_credentials_or_dns() {
    for valid in [
        "127.0.0.1:5454",
        "172.20.0.2:5454",
        "10.0.0.2:6000",
        "192.168.1.9:5454",
        "[::1]:5454",
        "[fd00::1]:5454",
    ] {
        assert!(Endpoint::parse(valid).is_ok(), "{valid}");
    }
    for invalid in [
        "NETRADIO:5454",
        "ws://172.20.0.2:5454/",
        "172.20.0.2:0",
        "8.8.8.8:5454",
        "0.0.0.0:5454",
        "169.254.169.254:80",
        "224.0.0.1:5454",
        "[::ffff:127.0.0.1]:5454",
        "[::]:5454",
    ] {
        assert_eq!(
            Endpoint::parse(invalid),
            Err(Error::InvalidEndpoint),
            "{invalid}"
        );
    }
}

#[test]
fn binary_headers_distinguish_iq_audio_and_secondary_and_reject_malformed_pairs() {
    let bytes = packet(1, &[i16::MIN, i16::MAX, 0, -1]);
    let BinaryFrame::Audio(audio) = decode_binary(&bytes).unwrap() else {
        panic!("wrong type");
    };
    assert_eq!(i16::from_le_bytes([audio[0], audio[1]]), i16::MIN);
    assert_eq!(i16::from_le_bytes([audio[2], audio[3]]), i16::MAX);
    assert!(matches!(
        decode_binary(&packet(2, &[1, 2])),
        Ok(BinaryFrame::Iq(_))
    ));
    assert_eq!(decode_binary(&packet(4, &[1, 2])), Ok(BinaryFrame::Other));
    assert_eq!(decode_binary(&packet(5, &[1, 2])), Ok(BinaryFrame::Other));
    for bytes in [
        vec![],
        vec![1],
        vec![2, 0, 1],
        vec![1, 0, 0, 0],
        vec![7, 0],
        vec![0; MAX_MESSAGE_BYTES + 1],
    ] {
        assert!(decode_binary(&bytes).is_err());
    }
}

#[test]
fn status_is_read_only_and_ignores_pushes_and_secondary_responses() {
    let (endpoint, handle) = fixture(|mut peer| {
        for (property, value) in [
            ("api_version", "1.0.3"),
            ("active_device", "FIXTURE nRSP-ST - NO RF"),
            ("valid_devices", "FIXTURE nRSP-ST - NO RF,Fixture B"),
            ("started", "True"),
            ("can_control", "False"),
            ("device_center_frequency", "100100000"),
            ("device_sample_rate", "2000000"),
            ("demodulator", "WFM"),
            ("valid_antennas", "Antenna A,Antenna B,Antenna C"),
            ("active_antenna", "Antenna B"),
        ] {
            let req = request(&mut peer);
            assert_eq!(req.event_type, "get_property");
            assert_eq!(req.property, property);
            assert_eq!(req.device.as_deref(), Some("primary"));
            assert!(req.value.is_empty());
            peer.send(Message::Text(
                format!(
                    r#"{{"event_type":"property_changed","property":"{property}","value":"wrong"}}"#
                )
                .into(),
            ))
            .unwrap();
            peer.send(Message::Text(format!(r#"{{"event_type":"get_property_response","device":"secondary","property":"{property}","value":"wrong"}}"#).into())).unwrap();
            response(&mut peer, property, value);
        }
    });
    let mut client = Client::connect(endpoint, &AtomicBool::new(false)).unwrap();
    let status = client.status(&AtomicBool::new(false)).unwrap();
    assert_eq!(status.active_device, "FIXTURE nRSP-ST - NO RF");
    assert_eq!(status.sample_rate_hz, 2_000_000.0);
    assert_eq!(status.antennas.len(), 3);
    assert_eq!(status.active_antenna, "Antenna B");
    assert!(status.started);
    assert!(!status.can_control);
    handle.join().unwrap();
}

#[test]
fn cancelled_iq_and_audio_subscriptions_disable_only_the_requested_client_stream() {
    for (kind, tag, ignored) in [(StreamKind::Iq, 2, 1), (StreamKind::Audio, 1, 2)] {
        let (endpoint, handle) = fixture(move |mut peer| {
            let req = request(&mut peer);
            assert_eq!(req.property, "started");
            response(&mut peer, "started", "true");
            let req = request(&mut peer);
            assert_eq!(req.event_type, kind.event());
            assert_eq!(req.value, "true");
            for tag in [ignored, 5, tag] {
                peer.send(Message::Binary(packet(tag, &[-100, 100, 0, -1]).into()))
                    .unwrap();
            }
            let req = request(&mut peer);
            assert_eq!(req.event_type, kind.event());
            assert_eq!(req.value, "false");
            // No device_stream_enable, retune, recording or unrelated stream command.
        });
        let cancelled = AtomicBool::new(false);
        let client = Client::connect(endpoint, &cancelled).unwrap();
        let mut callbacks = 0;
        let report = client
            .receive_for(kind, Duration::from_secs(1), &cancelled, |_| {
                callbacks += 1;
                cancelled.store(true, Ordering::Release);
            })
            .unwrap();
        assert_eq!(callbacks, 1);
        assert_eq!(report.sample_pairs, 2);
        assert!(report.cancelled);
        handle.join().unwrap();
    }
}

#[test]
fn malformed_ingress_unsubscribes_and_does_not_claim_samples() {
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        response(&mut peer, "started", "true");
        assert_eq!(request(&mut peer).value, "true");
        peer.send(Message::Binary(vec![2, 0, 1].into())).unwrap();
        let req = request(&mut peer);
        assert_eq!(req.event_type, "iq_stream_enable");
        assert_eq!(req.value, "false");
    });
    let cancel = AtomicBool::new(false);
    let client = Client::connect(endpoint, &cancel).unwrap();
    let result = client.receive_for(StreamKind::Iq, Duration::from_secs(1), &cancel, |_| {
        panic!("malformed data reached host")
    });
    assert!(matches!(result, Err(Error::Protocol(_))));
    handle.join().unwrap();
}

#[test]
fn silent_receive_reaches_deadline_unsubscribes_and_closes_the_owned_connection() {
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        response(&mut peer, "started", "true");
        assert_eq!(request(&mut peer).value, "true");
        let disable = request(&mut peer);
        assert_eq!(disable.event_type, "audio_stream_enable");
        assert_eq!(disable.value, "false");
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let client = Client::connect(endpoint, &cancel).unwrap();
    let report = client
        .receive_for(
            StreamKind::Audio,
            Duration::from_millis(20),
            &cancel,
            |_| panic!("silent fixture produced samples"),
        )
        .unwrap();
    assert_eq!(report.sample_pairs, 0);
    assert!(!report.cancelled);
    assert!(report.elapsed < Duration::from_secs(1));
    handle.join().unwrap();
}

#[test]
fn continuous_audio_consumer_can_stop_and_unsubscribe_without_touching_hardware() {
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        response(&mut peer, "started", "true");
        assert_eq!(request(&mut peer).event_type, "audio_stream_enable");
        peer.send(Message::Binary(packet(2, &[1, 2]).into()))
            .unwrap();
        peer.send(Message::Binary(packet(1, &[-100, 100]).into()))
            .unwrap();
        let stop = request(&mut peer);
        assert_eq!(stop.event_type, "audio_stream_enable");
        assert_eq!(stop.value, "false");
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let client = Client::connect(endpoint, &cancel).unwrap();
    let report = client
        .stream_audio(&cancel, |bytes| {
            assert_eq!(bytes.len(), 4);
            false
        })
        .unwrap();
    assert_eq!(report.sample_pairs, 1);
    assert!(!report.cancelled);
    handle.join().unwrap();
}

#[test]
fn unrelated_packets_cannot_keep_a_stalled_audio_session_alive() {
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        response(&mut peer, "started", "true");
        assert_eq!(request(&mut peer).event_type, "audio_stream_enable");
        peer.get_mut().set_nonblocking(true).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(Message::Text(text)) = peer.read() {
                let stop: Envelope = serde_json::from_str(&text).unwrap();
                assert_eq!(stop.event_type, "audio_stream_enable");
                assert_eq!(stop.value, "false");
                break;
            }
            assert!(
                Instant::now() < until,
                "audio subscription failed to time out"
            );
            peer.send(Message::Binary(packet(5, &[0, 0]).into()))
                .unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let cancel = AtomicBool::new(false);
    let client = Client::connect(endpoint, &cancel).unwrap();
    assert_eq!(
        client.stream_audio(&cancel, |_| panic!("secondary data became primary audio")),
        Err(Error::Timeout)
    );
    handle.join().unwrap();
}

#[test]
fn stopped_receiver_and_invalid_limits_do_not_enable_capture() {
    for duration in [Duration::ZERO, Duration::from_secs(6)] {
        let (endpoint, handle) = fixture(|mut peer| {
            assert!(peer.read().is_err());
        });
        let cancel = AtomicBool::new(false);
        let client = Client::connect(endpoint, &cancel).unwrap();
        assert!(matches!(
            client.receive_for(StreamKind::Iq, duration, &cancel, |_| {}),
            Err(Error::Protocol(_))
        ));
        handle.join().unwrap();
    }
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        response(&mut peer, "started", "false");
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let client = Client::connect(endpoint, &cancel).unwrap();
    assert_eq!(
        client.receive_for(StreamKind::Iq, Duration::from_secs(1), &cancel, |_| {}),
        Err(Error::NotStreaming)
    );
    handle.join().unwrap();
}

#[test]
fn deadline_and_cancellation_do_not_wait_for_missing_property_responses() {
    assert!(matches!(
        Client::connect(
            Endpoint::parse("127.0.0.1:9").unwrap(),
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
    let (endpoint, handle) = fixture(|mut peer| {
        assert_eq!(request(&mut peer).property, "started");
        std::thread::sleep(Duration::from_millis(80));
    });
    let cancel = AtomicBool::new(false);
    let mut client = Client::connect(endpoint, &cancel).unwrap();
    let start = Instant::now();
    assert_eq!(
        client.query("started", start + Duration::from_millis(20), &cancel),
        Err(Error::Timeout)
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    handle.join().unwrap();
}

#[test]
fn malformed_and_oversized_control_envelopes_fail_closed() {
    for text in ["{}".to_owned(), "x".repeat(MAX_CONTROL_BYTES + 1)] {
        let (endpoint, handle) = fixture(move |mut peer| {
            request(&mut peer);
            peer.send(Message::Text(text.into())).unwrap();
        });
        let cancel = AtomicBool::new(false);
        let mut client = Client::connect(endpoint, &cancel).unwrap();
        assert!(matches!(client.status(&cancel), Err(Error::Protocol(_))));
        handle.join().unwrap();
    }
}
