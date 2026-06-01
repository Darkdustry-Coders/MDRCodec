//! Properly typed encoder/decoders with a guaranteed synchronous API.

use std::io::{self, Seek, Write};

use crate::{enc::Encoder, opt::Compression};

/// A synchronous streaming encoder.
pub struct StreamingEncoder<W> {
    inner: Encoder<W>,
}
impl<W: Write> StreamingEncoder<W> {
    /// Create a new [StreamingEncoder] instance.
    pub fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .writeable()
            .compression(compression)
            .build()?;
        Ok(Self { inner: encoder })
    }

    /// Flush the buffers.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// An synchronous streaming encoder.
pub struct SeekingEncoder<W> {
    inner: Encoder<W>,
}
impl<W: Write + Seek> SeekingEncoder<W> {
    /// Create a new [SeekingEncoder] instance.
    pub fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .writeable()
            .seekable()
            .compression(compression)
            .build()?;
        Ok(Self { inner: encoder })
    }

    /// Flush the buffers.
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
