# Native nRSP-ST interoperability notes

The `nrsp-native` implementation was checked against the operator-owned
receiver on 2026-10-28. It implements the appliance's own browser transport at
TCP 9001. The device's native TCP 50000 interface is not used. No external
browser or desktop SDRconnect process participates in the Rust session.

The protocol was mapped from the publicly served browser client and validated
with independent native sessions. Inspection artifacts stayed in temporary
storage; vendor assemblies and decompiled sources are not part of this repo.
The decompressed `SDRconnect.WebBackend.wasm` examined had SHA-256
`5a6331bf0144e1c7e1b05278c13e852dc6b017a7429ad31eddc1d1038646ab0b`.
The managed assembly's informational revision was
`ce12bdd8e7ec3e67faa7d26166fe8cad6e912f2a`.
Fides Supra's earlier passive work established the endpoint and envelope shape;
this implementation additionally validates active commands and Opus ingress.

## Session and framing

- Connect a binary WebSocket to the explicitly selected private IPv4 peer.
- The first payload type is 201 (Hello). Its unpadded payload is 172 bytes and
  begins with `SDRP`. The observed protocol version is 0; other versions fail.
- Decode the four-byte big-endian header using the current header mask. The top
  byte is message type; bits 4–23 hold padded payload length; the low nibble holds
  padding length. Length must match the exact frame, be aligned to 16, and stay
  under the one-MiB message/frame limit.
- Server payloads other than raw I/Q types use the browser client's XTEA
  transformation: one eight-byte block per sixteen padded payload bytes,
  little-endian block words, 32 rounds. The first Hello uses public browser
  bootstrap constants. It supplies a connection-specific key and header mask
  for subsequent traffic. The Rust session has no Debug or serialization API.
- Hello word indices 2/3 carry center frequency, low word first; word 4 is
  sample rate; word 8 is antenna; the high byte of word 10 is the hardware model.
  Words 18–21 carry negotiated session key material, word 22 is the new mask,
  and word 23 contains permissions. No runtime key values are committed.
- Permission bit 0 allows WFM, bit 1 allows FM stereo, bit 5 allows audio, and bit 7 requires login.
  Required login fails before sending any control command in this initial client.
- The observed command payloads are big-endian words with zero padding and the
  negotiated masked header. The browser implementation does not encrypt these
  command payloads. This protocol is not presented as a secure Internet channel.

## Commands used

| Type | Meaning | Payload words |
| --- | --- | --- |
| 2 | Physical center frequency | low 32 bits, high 32 bits |
| 17 | Keepalive | Unix seconds low/high |
| 28 | Add connection-local virtual receiver | receiver ID 1 |
| 30 | Receiver start/stop | ID 1, boolean |
| 32 | Receiver frequency offset | ID 1, signed offset low/high; zero here |
| 33 | Receiver bandwidth | ID 1, Hz |
| 34 | Receiver demodulator | ID 1, mode code |
| 39 | Receiver audio gain | ID 1, percent; 100 with local output volume |
| 59 | Request device updates | zero |

Mode codes from the browser's shared contract are AM=1, USB=2, LSB=3, NFM=4,
WFM=5, CW=6, SAM=7. Raw mode is not exposed by this audio client. Antenna, RF gain,
bias-T and device hardware sample rate are not changed.

The appliance does not echo center-frequency commands to their sending
connection. When frequency changes, the client polls bounded, read-only Hello
connections until the appliance reports the requested value. All polling shares
one three-second deadline and checks cancellation; the temporary observer never
creates a virtual receiver. A stale first Hello is not treated as rejection or
confirmation. Displayed frequency comes from observed device state.

## Audio and lifecycle

Payload type 211 carries compressed audio: a big-endian receiver ID followed by
one Opus packet after removing envelope padding. Only ID 1 contributes audio.
The decoder produces interleaved float stereo at 48 kHz; its fixed output buffer
allows the maximum 120-ms Opus packet. Existing PCM conversion, bounded output
queue, local volume and native callback evidence are reused. Empty/truncated
packets and decoder failures end playback. Non-audio traffic does not extend the
three-second audio inactivity deadline. Keepalive is sent once per second.

One connection owns its virtual receiver. Live tuning is queued in a single
latest-value slot and executed between packets on the same worker. Stop mutes
local output through the existing engine, interrupts network polling, attempts
one bounded stop command for ID 1 and shuts down the owned socket. It does not
stop another client's virtual receiver. Physical tuning is shared and an
explicit tune can affect other clients.

Offline fixtures cover identity/version/framing, malformed messages, auth
requirements, command values/order, independent frequency confirmation, borrowed
Opus ingress and idempotent Stop/socket closure. Real device checks cover WFM
native callback output plus live NFM/WFM tuning. Extended link-loss recovery,
authenticated login, raw I/Q and other modes need additional work. Future
firmware changes may require a new protocol version implementation.

## FM audio properties

Type 40 carries twelve big-endian words (48 payload bytes). Word 0 is receiver
ID 1 and word 1 selects the property; only the selected property's value slot
is populated. The implemented property/slot pairs are filter enable (1/2),
limiter enable (2/3), stereo enable (5/6), de-emphasis (6/7), WFM low cut (7/8)
and WFM high cut (8/9). De-emphasis values are **0, 50 and 75**, as declared by
the browser's shared enum; they are not ordinal indexes 0/1/2. Filter frequencies
are in Hz. Commands are scoped to this connection's receiver 1.

Type 214 is an eight-byte receiver ID / PLL-locked boolean report. Only receiver
1 updates the native stereo status, and missing reports stay unknown. Retuning
or changing stereo selection clears previous lock evidence. Audio settings in
the status snapshot are the last successfully sent values, not readback from
the appliance. Raw two-channel Opus PCM does not establish stereo lock.

Fixtures verify every property word/slot, invalid filters, de-emphasis values,
receiver routing and malformed lock reports. A live muted session passed both
50/75 µs and mono/stereo changes while native callback playback continued; real
PLL lock/unlock reports were received and physical center frequency was unchanged.
