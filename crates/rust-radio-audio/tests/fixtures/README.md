# Synthetic validation fixtures

These files contain a generated 440 Hz sine wave, mono, 44.1 kHz, 0.25 seconds.
They contain no broadcaster recordings. CI reads the committed bytes; FFmpeg is
only needed to regenerate them:

```sh
ffmpeg -v error -f lavfi -i sine=frequency=440:sample_rate=44100 -t 0.25 -ac 1 -c:a libmp3lame -b:a 64k -write_xing 0 -id3v2_version 0 tone.mp3
ffmpeg -v error -f lavfi -i sine=frequency=440:sample_rate=44100 -t 0.25 -ac 1 -c:a aac -b:a 64k -f adts tone.aac
```

`prebuffer-stress.mp3` is a generated 440 Hz stereo sine wave, 44.1 kHz,
12 seconds, 128 kbps. It contains no broadcaster recording and is used by
`scripts/prebuffer_fixture_server.py` for the optional muted live-callback
prebuffer comparison. Regenerate it with:

```sh
ffmpeg -v error -f lavfi -i sine=frequency=440:sample_rate=44100 -t 12 -ac 2 -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0 prebuffer-stress.mp3
```

`prebuffer-trace.mp3` uses the same generated signal and encoding for 40
seconds. It has enough payload for replaying a bounded live response-body
timing trace. Regenerate it with:

```sh
ffmpeg -v error -f lavfi -i sine=frequency=440:sample_rate=44100 -t 40 -ac 2 -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0 prebuffer-trace.mp3
```

`wcrb-body-timing.json` and `skylab-body-timing.json` contain bounded samples
of response-body sizes and inter-chunk intervals observed by the transport
pump on 2026-10-28. They contain no stream URLs or audio. These are replay
fixtures, not packet captures or current network-health claims. The WCRB trace
has 76 samples/525,211 bytes; the Skylab trace has 206 samples/369,847 bytes.
