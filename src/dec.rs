//! MDR decoder.

use std::io::{self, Read, Seek};

use crate::{io::GenericIo, opt::Compression};

macro_rules! sread {
    ($self:expr) => {
        match $self.read.readable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'Read'",
                ))
            }
        }
    };
}
macro_rules! sseek {
    ($self:expr) => {
        match $self.read.seekable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'Seek'",
                ))
            }
        }
    };
}

#[cfg(feature = "futures")]
macro_rules! aread {
    ($self:expr) => {
        match $self.read.async_readable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'AsyncRead'",
                ))
            }
        }
    };
}
#[cfg(feature = "futures")]
macro_rules! aseek {
    ($self:expr) => {
        match $self.read.async_seekable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'AsyncSeek'",
                ))
            }
        }
    };
}

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
        #[cfg(feature = "futures")]
        if self.read.is_readable() && self.read.is_async_readable() && (self.read.is_seekable() != self.read.is_async_seekable()) {
            if self.read.is_seekable() {

            } else {

            }
        }

        let mut enc = Decoder { read: self.read };

        let mut buf = [0; 4];
        sread!(enc).read_exact(&mut buf)?;
        if &buf != b"MDR\0" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing file magic",
            ));
        }
        let compression = Compression::read(sread!(enc))?;

        Ok(enc)
    }
}
impl<R: Read> Builder<R> {
    pub const fn readable(mut self) -> Self {
        self.read.make_readable();
        self
    }
}
impl<R: Seek> Builder<R> {
    pub const fn seekable(mut self) -> Self {
        self.read.make_seekable();
        self
    }
}
#[cfg(feature = "futures")]
impl<R: futures_io::AsyncRead> Builder<R> {
    pub const fn async_readable(mut self) -> Self {
        self.read.make_async_readable();
        self
    }
}
#[cfg(feature = "futures")]
impl<R: futures_io::AsyncSeek> Builder<R> {
    pub const fn async_seekable(mut self) -> Self {
        self.read.make_async_seekable();
        self
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
