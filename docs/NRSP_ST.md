# SDRplay nRSP-ST access

## Direct native control and audio

The optional `nrsp-native` feature connects directly to the receiver on TCP
9001. It performs the native session handshake, creates its own virtual receiver,
sets physical center frequency and receiver demodulation/bandwidth, starts
reception, decodes 48 kHz stereo Opus and feeds the shared native audio output.
SDRconnect and a browser are not part of this playback path. The module does
not install software or change firmware on the appliance.

```sh
cargo run --locked -p rust-radio-cli --features nrsp-native -- \
  nrsp-native-status PRIVATE_IP:9001
cargo run --locked -p rust-radio-cli --features nrsp-native -- \
  nrsp-listen PRIVATE_IP:9001 99100000 wfm --seconds 30 --volume 25
cargo run --locked -p rust-radio-gui --features nrsp-native --bin rust-radio-gui
```

In the GUI, click **nSDR** to open setup and select or enter the receiver
`IP:9001`, frequency in Hz, mode and bandwidth, then press PLAY. During playback,
edit frequency/mode/bandwidth and press APPLY TUNING. The main source line shows
the observed center frequency. The address stays locked during playback. STOP
cancels playback, stops this connection's virtual receiver and closes its socket.
Physical center frequency is shared with other clients; an explicit tune affects
the appliance. Antenna, RF gain, bias-T and hardware sample rate are preserved.

Interactive CLI sessions accept `status`, `volume N`,
`tune FREQ_HZ MODE [BANDWIDTH_HZ]` and `stop`. Supported modes are `am`, `usb`,
`lsb`, `nfm`, `wfm`, `cw`, and `sam`; only WFM and NFM have current live mode
checks. Default bandwidths are 10 kHz AM/SAM, 3 kHz USB/LSB, 12 kHz NFM, 200 kHz
WFM and 500 Hz CW. The library permits explicit bandwidths from 100 to 250,000 Hz
and frequencies from 1 kHz to 2 GHz; the receiver and selected antenna still
determine actual reception.

The embedding API is `NativeRadioEngine::start_nrsp_native(peer, frequency,
mode, bandwidth, volume, output)`, followed by `tune_nrsp_native(...)`,
`nrsp_native_status()` and `nrsp_native_audio_stats()`. The last reports decoded
frames and pre-output signal peak; Playing still requires native callback
consumption. Hosts retain the same one-engine ownership and asynchronous Stop
contract. The lower-level `receiver::nrsp_native::Client` exposes status,
configuration, tuning, borrowed Opus packets and explicit stop without audio
dependencies.

This initial implementation targets the observed native browser protocol 0.
It validates Hello identity, envelope lengths, message types, padding, negotiated
session state and audio permission. The current firmware does not echo tuning
commands to their sending connection: changed frequency is confirmed through
bounded read-only Hello sessions, with cancellation and a three-second deadline.
Negotiated values remain private to the connection and are never logged or saved.
Inputs accept explicit private/loopback IPv4 peers; use a private network or VPN.
Password-protected native sessions are detected and rejected before any control
command; native login and raw I/Q are not implemented. Protocol failures require
explicit retry.

On 2026-10-28, the real receiver completed direct Rust handshake, WFM setup at
99.1 MHz and zero-volume Mac native callback playback. Within one session, live
tuning to 99.2 MHz NFM and back to 99.1 MHz WFM succeeded with independent
frequency confirmation. That run decoded 72,960 stereo frames with nonzero
signal. STOP closed the owned receiver session. This is live control, decode and
native callback evidence; a new audible listening check and physical Pi client
runtime remain open. Device addresses and runtime keys were not saved in the repo.

Implementation details and interoperability evidence are in
[the native protocol notes](NRSP_NATIVE_PROTOCOL.md).

## Advanced FM broadcast controls

Open **nSDR** and use these tabs:

- **Tune**: exact frequency entry (`99.5 MHz`, `99500 kHz` or `99500000`),
  selectable step sizes, mode, RF bandwidth and live Apply. Main-view arrows
  also step the active native receiver without replacing its audio worker.
- **FM Audio**: stereo/forced mono, 50/75 µs or disabled de-emphasis, audio
  filter enable, low cut (0–1000 Hz), high cut (3000–15000 Hz), and limiter.
  Defaults are stereo, 75 µs, 30–15000 Hz audio and limiter enabled.
- **Presets**: recall or save/update up to 16 named FM stations, including their
  audio settings and RF bandwidth. WCRB Boston at 99.5 MHz is included as a
  starting preset. It tunes the RF receiver; local antenna reception is required.
- **Device**: discovered receiver/address, output device and input scanning.

US FM defaults use 75 µs de-emphasis and 200 kHz stepping; 50 µs is available
for Europe and other regions. See [FM broadcast filtering reference](https://www.mathworks.com/help/comm/ref/fmbroadcastmodulatorbaseband.html).
Stereo lock is a separate receiver observation: unknown, unlocked or locked.
Forced mono is labelled independently. Settings show whether they were sent;
this firmware does not provide an audio-property readback acknowledgement.
The Opus stream remains two-channel even when mono is selected, so channel count
is never used to claim stereo lock.

```sh
cargo run --locked -p rust-radio-cli --features nrsp-native -- \
  nrsp-listen PRIVATE_IP:9001 99500000 wfm --stereo auto --deemphasis 75 \
  --low-cut 30 --high-cut 15000 --bandwidth 200000 --volume 25
# During an interactive native session:
# fm mono 50 150 8000
# fm auto 75 30 15000
```

`NativeRadioEngine::start_nrsp_native_fm` and `set_nrsp_native_fm` accept typed
`NativeFmSettings` / `NativeFmDeemphasis`. Invalid filters fail before I/O;
receiver stereo permission is checked before enabling it. FM changes use a
bounded latest-value slot on the same receiver worker. The lower-level client
API is `Client::set_fm`. Basic `start_nrsp_native` remains available for callers
that do not request FM audio settings.

The standalone host saves presets to `fm-presets.json` in the user's
`rust_radio` configuration directory: macOS Application Support, Windows
APPDATA, or Linux XDG_CONFIG_HOME / `~/.config`. `RUST_RADIO_PRESETS_PATH` can
select another file. Writes run off the UI thread and replace the file
atomically. The versioned JSON is limited to 16 KiB, contains no receiver address
or session key, and loading it never starts reception. Invalid input leaves the
in-memory bank intact. Embedded hosts import/export the bank through the panel
and own persistence themselves.

Live check on 2026-10-28: the existing physical frequency was preserved while
switching the connection-local receiver to mono / 50 µs / 150–8000 Hz, then
stereo / 75 µs / 30–15000 Hz. Native playback stayed Playing and decoded at least
71,040 stereo frames at zero output volume. Real stereo-lock reports included
both locked and unlocked states; the client does not manufacture a stable lock.
The owned session stopped cleanly. Audible quality and other regional signals
need separate listening checks.

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
audio and tuning are now provided separately by the `nrsp-native` feature above.

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
