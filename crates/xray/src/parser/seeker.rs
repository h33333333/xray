use std::io::{Read, Seek};

/// A helper struct that is needed to allow reconstructing the [tar::Archive] multiple times during the image parsing.
///
/// When we create a new [tar::Archive], it also starts tracking the current position of the cursor in the struct and sets the current position to `0`,
/// which might contradict with the actual state of the cursor if you've used the same reader was used already by some other [tar::Archive] instance,
/// as [tar::Archive] doesn't expect the archive to be reconstructed multiple times during the execution.
///
/// In this case, we need to reconstruct the archive object multiple times during parsing,
/// as that's required to efficiently parse the nested TAR archives (i.e. the layers) of an Image.
///
/// To fix the above, we need to either stop updating the position in [tar::Archive] to the one returned by [std::io::Seek::seek], which requires changing the [tar] crate OR
/// use a helper struct like this one which can normalize the position using offsets when it's required.
pub struct SeekerWithOffset<S> {
    inner: S,
    pos: u64,
    pos_offset: u64,
}

impl<R: Read> Read for SeekerWithOffset<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<S: Seek> Seek for SeekerWithOffset<S> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = self.inner.seek(pos)?;

        // Don't allow seeking before the current offset
        if new_pos < self.pos_offset {
            return Err(std::io::Error::other("seek position out of bounds"));
        }

        self.pos = new_pos;
        let adjusted_pos = self.pos - self.pos_offset;
        Ok(adjusted_pos)
    }
}

impl<S> SeekerWithOffset<S> {
    pub fn new(inner: S) -> Self {
        SeekerWithOffset {
            inner,
            pos: 0,
            pos_offset: 0,
        }
    }

    /// Uses the current cursor's position as the new offset.
    ///
    /// It will use this offset to adjust the position returned by [Seek::seek].
    pub fn mark_offset(&mut self) {
        self.pos_offset = self.pos;
    }
}
