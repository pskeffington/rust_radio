# Optional SDR receive adapter

For the selected **nRSP-ST network receiver**, use the separate
[SDRconnect integration](NRSP_ST.md). It does not use the USB adapter below.

The standalone module keeps IP radio operational without an SDR SDK or device.
Enable `soapy-sdr` only on a machine with SoapySDR and the appropriate receiver
driver installed. The current Rust binding requires Rust 1.85 or newer. No
transmit, arbitrary driver-argument or RF replay API is exposed by this module.

```sh
cargo run -p rust-radio-cli --features soapy-sdr -- sdr-devices
cargo run -p rust-radio-cli --features soapy-sdr -- \
  sdr-capture DEVICE_ID 100100000 1024000 --milliseconds 500
```

Copy `DEVICE_ID` from a fresh discovery. Discovery does not activate a receive
stream. Opening a selected handle queries real RX channels, frequency ranges,
sample-rate ranges/steps, gain bounds and CS16 format support. The chosen rate
must be in those device-reported ranges; no simulator capabilities are applied
to physical devices. Discovery IDs are tied to canonical driver descriptors;
duplicates fail closed and disconnected IDs must be refreshed.

`SoapyReceiver` implements the shared `RadioDevice` contract and can be owned by
`RadioTuner`. It advertises the explicitly selected sample rate for that handle;
`sample_rate_ranges()` reports the full normalized driver ranges. Tuning opens
exactly one receive stream after bounds checks. Driver reads request at most
8,192 complex samples with a 100 ms timeout. A timeout yields no samples; other
read failures close the stream. Retuning closes the old stream first.

`receiver::capture()` owns a receiver for an explicit I/Q health check. Its data
buffer is capped at 16,384 signed interleaved values, duration is limited to five
seconds, cancellation is checked between reads, and completion/error stops the
receiver. Captured data is discarded. Run synchronous discovery/open/capture on
a host worker: device initialization and teardown depend on the native driver.
The read timeout and capture duration do not impose a deadline on driver setup.

**Current boundary:** this is an I/Q receive adapter, not an RF audio player.
AM/FM/SSB demodulation is not implemented; physical tuning accepts RawIq only.
Sample ingress does not prove a station signal, audible reception, antenna
quality or frequency calibration. Simulated receivers are explicitly marked;
unknown implementations default to Unspecified rather than Physical.

Offline tests exercise cancellation, tuning failure, device loss, malformed I/Q
counts, buffer bounds, cleanup, descriptor identity and capability validation.
Native-feature tests link the SDK but do not enumerate/open hardware. A separate
explicit discovery on this Mac found no device factories or receivers. Actual
RF capture, RF audio, device unplug/replug and Raspberry Pi hardware acceptance
remain unverified until a supported receiver and driver are available.

The shared tuner was extracted from pip_boy commit
`23f9e2bd362fd7984963cfa9bb7f4380bb5566f5`, file blob
`6736d116920a52988fe7fffca1d6096e7330a89b`. Host compatibility paths re-export
the standalone implementation instead of copying it.
