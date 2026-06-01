//! MDR decoder.

use std::io;

use crate::io::GenericIo;

pub struct Builder<R> {
    read: GenericIo<R>,
}
impl<R> Builder<R> {
    #[must_use]
    pub fn new(read: R) -> Self {
        Self {
            read: GenericIo::new_empty(read),
        }
    }

    pub fn build(self) -> io::Result<Decoder<R>> {
        todo!();
        // let mut enc = Decoder { read: self.read };

        // let mut buf = [0; 4];
        // enc.read.read_exact(&mut buf)?;
        // if &buf != b"MDR\0" {
        //     return Err(io::Error::new(
        //         io::ErrorKind::InvalidData,
        //         "Missing file magic",
        //     ));
        // }

        // Ok(enc)
    }
}

/// MDR decoder.
pub struct Decoder<R> {
    /// Writing destination.
    read: GenericIo<R>,
}
impl<R> Decoder<R> {
    /// Create a new [Builder].
    #[must_use]
    #[inline(always)]
    pub fn builder(write: R) -> Builder<R> {
        Builder::new(write)
    }

    /// Create a streaming [Encoder].
    ///
    /// This is equivalent to `Encoder::builder(write).build()`
    #[inline(always)]
    pub fn streaming(write: R) -> io::Result<Self> {
        Decoder::builder(write).build()
    }
}
