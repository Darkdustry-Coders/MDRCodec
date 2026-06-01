//! Properly typed encoder/decoders with a guaranteed synchronous API.

use std::io::{self, Write};

use crate::{enc::Encoder, opt::Compression};

/// A synchronous streaming encoder.
pub struct StreamingEncoder<W> {
    inner: Encoder<W>,
}
impl<W: Write> StreamingEncoder<W> {
    pub fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .writeable()
            .compression(compression)
            .build()?;
        Ok(Self { inner: encoder })
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
