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
