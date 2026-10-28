use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Mutex;
use symphonia::core::io::MediaSource;

/// Non-seekable source shared by playback and silent validation.
/// Only decoder workers access it, never the real-time audio callback.
pub(crate) struct LiveMediaSource<R: Read + Send> {
    reader: Mutex<R>,
}

impl<R: Read + Send> LiveMediaSource<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self {
            reader: Mutex::new(reader),
        }
    }
}

impl<R: Read + Send> Read for LiveMediaSource<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader
            .get_mut()
            .map_err(|_| io::Error::other("Audio source lock poisoned"))?
            .read(bytes)
    }
}

impl<R: Read + Send> Seek for LiveMediaSource<R> {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Live broadcasts are not seekable",
        ))
    }
}

impl<R: Read + Send> MediaSource for LiveMediaSource<R> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// Symphonia 0.5's ADTS reader matches only MPEG-4's 0xfff1 sync word.
/// MPEG-2 AAC-LC carries the same AAC payload but sets the ADTS ID bit.
/// Normalize that bit only at parsed, unprotected ADTS frame boundaries;
/// payload bytes, including sync-like byte sequences, are never modified.
pub(crate) struct AdtsCompat<R: Read> {
    inner: io::BufReader<R>,
    header: [u8; 7],
    header_pos: usize,
    remaining: usize,
}

impl<R: Read> AdtsCompat<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self {
            inner: io::BufReader::new(reader),
            header: [0; 7],
            header_pos: 7,
            remaining: 0,
        }
    }
}

impl<R: Read> Read for AdtsCompat<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.header_pos < 7 {
            let count = (7 - self.header_pos).min(out.len());
            out[..count].copy_from_slice(&self.header[self.header_pos..self.header_pos + count]);
            self.header_pos += count;
            return Ok(count);
        }
        if self.remaining > 0 {
            let limit = self.remaining.min(out.len());
            let count = self.inner.read(&mut out[..limit])?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Truncated ADTS frame",
                ));
            }
            self.remaining -= count;
            return Ok(count);
        }
        // A live stream may begin in the middle of an ADTS frame.
        let mut previous = 0u8;
        loop {
            let mut byte = [0];
            match self.inner.read_exact(&mut byte) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(0),
                Err(e) => return Err(e),
            }
            if previous == 0xff && (byte[0] == 0xf9 || byte[0] == 0xf1) {
                self.header[0] = 0xff;
                self.header[1] = 0xf1;
                self.inner.read_exact(&mut self.header[2..])?;
                let length = ((self.header[3] as usize & 3) << 11)
                    | ((self.header[4] as usize) << 3)
                    | ((self.header[5] as usize) >> 5);
                if length < 7 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid ADTS frame length",
                    ));
                }
                self.remaining = length - 7;
                self.header_pos = 0;
                return self.read(out);
            }
            previous = byte[0];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_media_source_does_not_offer_seeking() {
        let mut source = LiveMediaSource::new(io::Cursor::new(vec![1u8, 2, 3]));
        let mut bytes = [0u8; 2];
        assert_eq!(source.read(&mut bytes).unwrap(), 2);
        assert_eq!(bytes, [1, 2]);
        assert!(!source.is_seekable());
        assert!(source.seek(SeekFrom::Start(0)).is_err());
    }
}
