# SDRplay nRSP-ST access

## Direct device status

The `sdr` feature can check the receiver's own network services without
SDRconnect, using the direct-service findings from Fides Supra main `2447fdc`:

```sh
cargo run --locked -p rust-radio-cli --features sdr -- nrsp-status PRIVATE_IPV4
```

This explicit, bounded check requests the built-in web service on TCP 9001 and
opens/closes TCP 50000 without a device command. It accepts literal private or
loopback IPv4 addresses, does not follow HTTP redirects, and never scans a LAN.
The library API is `rust_radio::receiver::nrsp::Host`. An open service indicates
reachability only; it does not prove identity, tuning, I/Q, or audio. Fides
Supra's proven native display path uses a passive browser-session tap and does
not document a native audio or receiver-control client. Direct native nRSP-ST
audio and tuning are therefore still unimplemented.

On 2026-10-27, the operator-identified NETRADIO host returned HTTP 200 on port
9001 and accepted TCP 50000 when checked directly by this CLI. No radio command
or audio subscription was sent. Device addresses remain local configuration.

## SDRconnect audio adapter

The first network receiver target is the nRSP-ST. For third-party audio SDRplay directs this model to
the [SDRconnect WebSocket API](https://sdrplay.com/websocket-api/), rather than the
[USB hardware API](https://sdrplay.com/hardware-api/). Pip-Boy consumes the same
standalone module; no receiver implementation is copied into the host.

Run SDRconnect (GUI or headless) on the Mac or Raspberry Pi, connect it to the
nRSP-ST, and enable its WebSocket server. Point this module at that SDRconnect
host's API port, usually 5454. The radio's own network service is a different
protocol. A device named NETRADIO on a router is not proof of a running API.
SDRconnect setup, receiver selection and device-list refresh remain operator
actions. Use **Refresh Device List** and select the nRSP-ST in IQ Lite mode
there. The refresh is an SDRconnect UI action, not a WebSocket command sent by
the module.

```sh
# Independent of SoapySDR and the USB SDRplay SDK:
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-status 127.0.0.1:5454

# Capture is passive and needs an already running receiver. Both commands
# discard samples:
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-capture 127.0.0.1:5454 iq --milliseconds 500
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-capture 127.0.0.1:5454 audio --milliseconds 500
```

Use an explicit private-LAN or loopback IP and port; no DNS, public-endpoint
fallback, credentials, discovery scan or redirect is used. This is a plain
WebSocket connection to the operator-selected local API. No endpoint is saved.

`rust_radio::receiver::sdrconnect::Client` provides a read-only status snapshot,
`receive_for()` and explicit `start_receiver()`. Status reports server-supplied
receiver/tuning/antenna data. Receive and capture are passive: they subscribe to
the already running primary receiver, deliver borrowed packets to a host
callback, disable that subscription, and close their owned connection. A new
receive session needs a new client, so buffered packets cannot leak into a later
session. Explicit native Play may start the already selected receiver if
SDRconnect reports `can_control`; it sends the documented
`device_stream_enable=true` event and waits for `started=true` before opening
audio output. It never selects or refreshes devices, and stopping local playback
leaves shared reception running. If SDRconnect denies control, Play reports an
error and does not open the output.

Startup status is polled at most ten times per second with cancellation checked
between polls. If startup is not confirmed within three seconds, `StartTimeout`
is terminal for native playback: it reports Refresh Device List/selection/retry
guidance without entering network recovery or issuing another activation. The
deadline also covers a missing startup response; transport disconnections retain
normal bounded recovery. Local Stop can cancel the startup wait and leaves shared
reception running if activation already reached the receiver.

The implementation follows [API 1.0.3](https://sdrplay.com/wp-content/uploads/2026/08/SDRconnect_WebSocket_API.pdf).
Primary stereo audio is signed little-endian 16-bit PCM at 48 kHz; I/Q uses
signed little-endian pairs. Packet tags distinguish those from secondary-device
and spectrum data. The capture CLI never opens an audio output. Native playback is now available
through the shared `NativeRadioEngine::start_sdrconnect()` entry point when both
`ip-radio` (default) and `sdrconnect` are enabled. It reuses CPAL output selection,
volume, bounded PCM conversion/queueing, retries and asynchronous Stop. A new
session starts reception on the already selected receiver when SDRconnect grants
control; the engine does not select, refresh or retune shared hardware.

```sh
cargo run --locked -p rust-radio-cli --features sdrconnect -- \
  sdrconnect-listen 127.0.0.1:5454 --seconds 10 --volume 10
```

`Client::stream_audio()` supports continuous playback with a three-second
primary-audio inactivity limit. Other channels and spectrum/control traffic do
not extend that deadline; empty primary packets do not extend it either. Time
spent delivering PCM to the native output worker does not consume the next
network read's timeout, so output backpressure on a large packet does not
trigger a false reconnect. Fixed-duration captures retain their original read
deadline. Stereo i16 PCM is converted/resampled on the worker;
Playing requires samples consumed by the native callback. Stop mutes queued
samples immediately, unsubscribes and closes the owned connection. Hosts retain
the engine and poll `try_finish()`/`shutdown_pending()` before replacing it.

Limits: one MiB per WebSocket message/frame, 8 KiB control envelopes, three-second
connection/handshake and status deadlines, 1–5000 ms receive windows, and a 250 ms
unsubscribe attempt followed by socket closure. Cancellation is checked between
nonblocking operations; the initial TCP connect can take up to three seconds.
Run on a host worker and keep callbacks brief. No samples are recorded or server
recordings started. Apart from explicit Play activation, no shared-device control
commands are sent.

Local fixtures verify read-only requests, explicit receiver activation,
type/channel separation, cancellation, malformed/oversized data, timeout,
stopped-receiver errors and unsubscribe/close.
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
Raw I/Q and Pi runtime acceptance remain open. USB Soapy discovery is not an
nRSP-ST acceptance test.

Native audio tests cover signed endianness, callback evidence, output failure,
malformed packets, cancellation and resource cleanup. An earlier live attempt
was rejected while SDRconnect was stopped, correctly failing before opening
native output. The operator identified Refresh Device List followed by Start
as the required setup step.

On 2026-10-27, with the real nRSP-ST running in IQ Lite at its existing WFM
tuning, standalone native playback passed a five-second check at 10% volume.
The SDR-enabled Pip-Boy executable also reached Playing through native callback
consumption using temporary preferences. An independent three-second standalone
subscriber succeeded while Pip-Boy stayed Playing. Volume adjustment, output
restart, duplicate-start rejection and asynchronous Stop passed. One observed
Stop response was 9.44 ms, not a timing guarantee; worker cleanup completed and
read-only status confirmed the shared receiver remained running. A fresh 500 ms
subscription then received 26,409 stereo frames. No received samples were saved,
and no tuning, firmware or shared receiver start/stop commands were sent by these
checks. This verifies real-receiver PCM through native output callbacks.

The live checks were repeated against module `6997dff` and Pip-Boy `f71dd2d`
after the operator refreshed and started SDRconnect again: PCM ingress resumed,
both native players passed, concurrent reception and output restart passed,
and Stop completed cleanup while leaving shared reception running. The observed
Stop response was 11.52 ms; a fresh 500 ms capture received 24,008 stereo frames.

On 2026-10-27 the operator confirmed hearing radio audio on the MacBook Air
speakers using the system default audio output. Speaker audibility is confirmed
for this Mac run; RF signal quality has not been measured.

A separate synthetic-PCM native-output smoke passed on macOS after explicitly
pausing CPAL's CoreAudio stream before prebuffering. Both standalone and embedded
players reached Playing from actual native callback consumption, native output
restart preserved the receiver source, Stop released subscriptions, and switching
to live Skylab worked. One observed host Stop response was about 50 ms. These
are native pipeline checks, not a physical nRSP-ST listening confirmation.
