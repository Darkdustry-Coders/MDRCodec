//! Properly typed encoder/decoders with a guaranteed synchronous API.

use std::{io::{self, Read, Seek, Write}, mem::transmute};

use crate::{data::{Chunk, WorldAccess}, dec::Decoder, enc::Encoder, io::TryClone, opt::Compression};

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

    /// Write a MAP chunk.
    pub fn write_map<WA: WorldAccess>(&mut self, map: WA) -> io::Result<()> {
        self.inner.write_map(map)
    }

    /// Write MAP chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid MAP chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_map_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_map_raw(map)
        }
    }
    
    /// Write ID chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_id_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_id_raw(map)
        }
    }

    /// Flush the buffers.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A synchronous seeking encoder.
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

    /// Write a MAP chunk.
    pub fn write_map<WA: WorldAccess>(&mut self, map: WA) -> io::Result<()> {
        self.inner.write_map(map)
    }

    /// Write MAP chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid MAP chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_map_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_map_raw(map)
        }
    }
    
    /// Write ID chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_id_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_id_raw(map)
        }
    }

    /// Flush the buffers.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A synchronous seeking encoder with try_clone support.
pub struct CloningEncoder<W> {
    inner: Encoder<W>,
}
impl<W: TryClone + Write + Seek> CloningEncoder<W> {
    /// Create a new [SeekingEncoder] instance.
    pub fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .writeable()
            .seekable()
            .try_cloneable()
            .compression(compression)
            .build()?;
        Ok(Self { inner: encoder })
    }

    /// Write a MAP chunk.
    pub fn write_map<WA: WorldAccess>(&mut self, map: WA) -> io::Result<()> {
        self.inner.write_map(map)
    }
    
    /// Write MAP chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid MAP chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_map_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_map_raw(map)
        }
    }
    
    /// Write ID chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_id_raw(&mut self, map: &[u8]) -> io::Result<()> {
        unsafe {
            self.inner.write_id_raw(map)
        }
    }

    /// Flush the buffers.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl<W: TryClone + Write + Seek> AsRef<SeekingEncoder<W>> for CloningEncoder<W> {
    fn as_ref(&self) -> &SeekingEncoder<W> {
        // Safety: Both have the same exact layout. The only difference is that SeekingEncoder
        //         doesn't have try_cloneable set.
        unsafe {
            transmute(self)
        }
    }
}

/// A synchronous streaming decoder.
pub struct StreamingDecoder<R> {
    inner: Decoder<R>,
}
impl<R: Read> StreamingDecoder<R> {
    /// Create a new [StreamingDecoder] instance.
    pub fn new(read: R) -> io::Result<Self> {
        let encoder = Decoder::builder(read)
            .readable()
            .build()?;
        Ok(Self { inner: encoder })
    }
}
impl<R: Read> Iterator for StreamingDecoder<R> {
    type Item = io::Result<Chunk>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
