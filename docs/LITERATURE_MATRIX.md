# Rust radio literature / engineering decision matrix

Status: research-backed architectural candidate list, 2026-10-26. **Source presence is not hardware validation or a license grant.** Recheck crate versions and licenses before dependency pins.

| Project / source | Relevant evidence | rust_radio decision | Validation gate |
| --- | --- | --- | --- |
| [RustAudio CPAL](https://github.com/RustAudio/cpal) | Native audio device enumeration, host APIs, stream callbacks; Linux ALSA development files required | Direct output backend; callback must avoid blocking/allocations/network | macOS CoreAudio, Linux ARM64 ALSA, Windows WASAPI |
| [Symphonia](https://docs.rs/symphonia/latest/symphonia/) | Pure Rust decoding/demuxing; MP3 and AAC require explicit features | Decoder worker with narrow format features | MP3/AAC offline fixtures and live samples |
| [Rodio](https://github.com/RustAudio/rodio) | Higher level player on CPAL/Symphonia | Comparison candidate, not default engine | Device-switch and cancellation spike |
| [whirr](https://github.com/samuelb/whirr) | Internet radio architecture reference | Study bounded HTTP/ICY queues, cancellation; independently implement | Fault-injection streaming |
| [reqwest](https://docs.rs/reqwest/latest/reqwest/) | Rust HTTP client | HTTPS-only pinned publisher origin policy; redirects explicitly checked | SSRF, redirects, retries and inactivity timeouts |
| [SoapySDR](https://docs.rs/soapysdr/latest/soapysdr/) | SDR discovery and IQ access through native drivers | Optional feature-gated receive-only hardware bridge | Physical RTL-SDR and second receiver |
| [rustfft](https://github.com/ejmahler/RustFFT) | Rust FFT processing | Spectrum/waterfall analysis research candidate | Deterministic IQ/FFT fixtures |
| [rubato](https://github.com/HEnquist/rubato) | Audio sample-rate conversion | Compare resampling quality and bounded latency | Quality, clipping, ARM CPU and power tests |
| [RTL-SDR](https://github.com/osmocom/rtl-sdr) | Widely deployed receive-only USB hardware ecosystem | Initial device test matrix | Hotplug, ppm correction, samples/drop rate |
| [jasonherald/rtl-sdr](https://github.com/jasonherald/rtl-sdr) | Multi-crate Rust radio/DSP architecture | Reference modular DSP/receiver separation; assess implementation/license independently | Architectural comparison |
| [Rust-Boy native RADIO roadmap](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_NATIVE_ROADMAP.md) | Existing user-tested native IP path, sound output, HUD and tests | Port with provenance and behavior parity, **do not rewrite blind** | Golden-master parity against pinned upstream SHA |
| [Rust-Boy SDR contract](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_SDR.md) | Tuning validation and honest simulated backend | Extract source-neutral receiver interface | Pure tuner tests and RF hardware evidence |
| [SomaFM Left Coast 70s direct stream guidance](https://somafm.com/seventies/directstreamlinks.html) | Permanent 128 kbps MP3 M3U is `https://somafm.com/m3u/seventies.m3u`; publisher limits streams to individual personal use | Use publisher-listed URL; receiver-side use only, no redistribution or relay | Periodic bounded decode; written product-distribution rights check |
| [Skylab Radio listen page](https://www.skylabradio.com/listen) | Publisher provenance for known working operator demo | Preserve station reference, dynamically validate endpoint | Live audio + failover test |
| [NPR SmartAudio guide](https://media.npr.org/documents/about/smart%20audio/SmartAudio_Alexa.pdf), [CDS audio profile](https://npr.github.io/content-distribution-service/api-reference/profiles/audio.html) and [premium-content guide](https://npr.github.io/content-distribution-service/legal/premium-content/index) | SmartAudio distinguishes NPR One's personalized national/local news mix from live local member-station streams. CDS uses `isStreamable` for on-air audio and identifies premium enclosure relations. | “NPR News” does not identify a single continuous national station stream in these sources. Require a named member station, first-party endpoint and applicable rights; filter premium assets unless explicitly authorized. | Verify the specific broadcaster's stable live endpoint and reuse terms; bounded decode test |

## Evidence hierarchy
1. Official API documentation and source code.
2. Reproducible lab fixture and tests.
3. Mac / Raspberry Pi / Windows field reports tied to exact hardware and revision.
4. Community examples as design leads only.

## Security and portability controls
- No automatic station streaming, RF capture, or microphone access on import/launch.
- Curated presets retain publisher endpoint authorization. A separate explicitly entered direct-stream path permits HTTPS and private/loopback literal IPv4 HTTP; it does not use proxies or follow redirects. Hosts should treat entered URLs as trusted operator input and protect query tokens.
- All buffers, playlist sizes, decoded frames, retry budgets and metadata are bounded.
- One active receiver, monotonic generation IDs and actual worker cancellation.
- Satellite is receive-only, with protocol-specific permission and regulatory review. Do not automatically decode private traffic.
- Default workspace builds without native SDR libraries or network/audio dependencies. Feature-gate integrations.
- CI: fmt, clippy, test, dependency advisories and license review; add hardware-in-loop acceptance separately.
