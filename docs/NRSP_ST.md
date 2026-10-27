# SDRplay nRSP-ST via SDRconnect

The first network receiver target is the nRSP-ST. SDRplay directs this model to
the [SDRconnect WebSocket API](https://sdrplay.com/websocket-api/), rather than the
[USB hardware API](https://sdrplay.com/hardware-api/). Pip-Boy consumes the same
standalone module; no receiver implementation is copied into the host.

Run SDRconnect (GUI or headless) on the Mac or Raspberry Pi, connect it to the
nRSP-ST, and enable its WebSocket server. Point this module at that SDRconnect
host's API port, usually 5454. The radio's own network service is a different
protocol. A device named NETRADIO on a router is not proof of a running API.
SDRconnect setup and receiver selection remain operator actions.

```sh
# Independent of SoapySDR and the USB SDRplay SDK:
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-status 127.0.0.1:5454

# Select/start the receiver in SDRconnect first. Both commands discard samples:
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-capture 127.0.0.1:5454 iq --milliseconds 500
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-capture 127.0.0.1:5454 audio --milliseconds 500
```

Use an explicit private-LAN or loopback IP and port; no DNS, public-endpoint
fallback, credentials, discovery scan or redirect is used. This is a plain
WebSocket connection to the operator-selected local API. No endpoint is saved.

`rust_radio::receiver::sdrconnect::Client` provides a read-only status snapshot
and `receive_for()`. Status reports server-supplied receiver/tuning/antenna data;
it does not select, start, stop or retune shared hardware. Receive subscribes to
the already running primary receiver, delivers borrowed packets to a host
callback, disables that subscription, and closes its owned connection. A stopped
receiver yields an explicit error. A new receive session needs a new client, so
buffered packets cannot leak into a later session.

The implementation follows [API 1.0.3](https://sdrplay.com/wp-content/uploads/2026/08/SDRconnect_WebSocket_API.pdf).
Primary stereo audio is signed little-endian 16-bit PCM at 48 kHz; I/Q uses
signed little-endian pairs. Packet tags distinguish those from secondary-device
and spectrum data. This adds network ingress, not an independent RF demodulator
or native RF audio player. The capture CLI never opens an audio output.

Limits: one MiB per WebSocket message/frame, 8 KiB control envelopes, three-second
connection/handshake and status deadlines, 1–5000 ms receive windows, and a 250 ms
unsubscribe attempt followed by socket closure. Cancellation is checked between
nonblocking operations; the initial TCP connect can take up to three seconds.
Run on a host worker and keep callbacks brief. No samples are recorded and no
server recording or shared-device streaming commands are sent.

Local fixtures verify read-only requests, type/channel separation, cancellation,
malformed/oversized data, timeout, stopped-receiver errors and unsubscribe/close.
Initial API checks on the supplied Pi address and localhost were refused.
Later, SDRconnect 1.0.10 on the Mac exposed API 1.0.3 and the selected nRSP-ST.
Read-only status succeeded both stopped and streaming. With the receiver started
in IQ Lite mode at its existing WFM tuning, a two-second audio subscription
received 96,030 stereo frames and unsubscribed. No audio was saved or played by
this CLI; this is network PCM ingress, not a listening confirmation.

I/Q subscriptions returned zero pairs in IQ Lite mode. Selecting Full IQ and
attempting to start displayed SDRconnect's “Update Required” firmware notice;
that mode could not be validated. No firmware was modified. The original IQ Lite
selection and mute setting were restored, and the receiver was left stopped.
Raw I/Q, native RF audio output, shared-client and Pi runtime acceptance remain
open. USB Soapy discovery is not an nRSP-ST acceptance test.
