use std::{
    borrow::Cow,
    io::{self, Seek, Write},
    marker::PhantomData,
    time::{Duration, Instant},
};

#[cfg(feature = "futures")]
use futures_io::{AsyncSeek, AsyncWrite};
#[cfg(feature = "futures")]
use futures_util::{AsyncSeekExt, AsyncWriteExt};

use crate::{
    data::ChunkKind,
    io::{GenericIo, TryClone},
    opt::Compression,
};

macro_rules! swrite {
    ($self:expr) => {
        match $self.write.writeable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'Write'",
                ))
            }
        }
    };
}
macro_rules! sseek {
    ($self:expr) => {
        match $self.write.seekable() {
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
macro_rules! awrite {
    ($self:expr) => {
        match $self.write.async_writeable() {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'AsyncWrite'",
                ))
            }
        }
    };
}
#[cfg(feature = "futures")]
macro_rules! aseek {
    ($self:expr) => {
        match $self.write.async_seekable() {
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

/// A lot of zeroes to write the entire JMP table in one go.
///
/// If the configured JMP table needs less zeroes than this buffer provides,
/// it will not allocate a new buffer.
static ZEROES: &[u8] = &[0;
    (8 /* Timestamp (in millis) */ + 8 /* File pointer */ + 1/* Entry validity */) * 1024
    + 8 /* File pointer to the next JMP chunk */ + 1 /* Entry validity */];

/// Encoder builder.
pub struct Builder<W> {
    write: GenericIo<W>,
    snapshot_duration: Duration,
    jmptable_size: usize,
    jmptable_waittime: u64,
    mod_buffer_size: usize,
    compression: Compression,

    _phantom: PhantomData<W>,
}
impl<W> Builder<W> {
    #[must_use]
    pub fn new(write: W) -> Self {
        Self {
            write: GenericIo::new_empty(write),
            snapshot_duration: Duration::from_secs(10),
            jmptable_size: 1024,
            jmptable_waittime: 1000,
            mod_buffer_size: 1024 * 64,
            compression: Compression::None,

            _phantom: PhantomData,
        }
    }

    /// Specify how long will it take until a new MAP chunk and a
    /// JMP entry is created.
    ///
    /// Only takes effect if this [Encoder] is either seekable or
    /// try-cloneable.
    #[must_use]
    pub fn snapshot_duration(mut self, duration: Duration) -> Self {
        self.snapshot_duration = duration;
        self
    }

    /// Amount of entries that are contained within one jump table in
    /// entry count.
    ///
    /// If parameter is smaller than 1, that is used instead.
    ///
    /// Higher values will increase the amount of chunks, the
    /// buffer size necessary to read a JMP chunk in one go, and
    /// encoder memory usage as it'll need to allocate a buffer
    /// of zeroes.
    ///
    /// Only takes effect if this [Encoder] is either seekable or
    /// try-cloneable.
    #[must_use]
    pub fn jmptable_size(mut self, size: usize) -> Self {
        self.jmptable_size = size.max(1);
        self
    }

    /// Size of the modifications buffer in bytes.
    ///
    /// If parameter is smaller than 1024, that is used instead.
    #[must_use]
    pub fn mod_buffer_size(mut self, size: usize) -> Self {
        self.mod_buffer_size = size.max(1024);
        self
    }

    /// Set the compression for this encoder.
    #[must_use]
    pub fn compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Enable the use of lz4 compression algorithm.
    ///
    /// `lz4` feature must be enabled.
    #[must_use]
    #[cfg(feature = "lz4")]
    pub fn lz4(mut self, mode: lz4::block::CompressionMode) -> Self {
        self.compression = Compression::Lz4 { mode };
        self
    }

    /// Enable the use of deflate compression algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[must_use]
    #[cfg(feature = "flate2")]
    pub fn deflate(mut self, quality: flate2::Compression) -> Self {
        self.compression = Compression::Deflate { quality };
        self
    }

    /// Enable the use of zlib compression algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[must_use]
    #[cfg(feature = "flate2")]
    pub fn zlib(mut self, quality: flate2::Compression) -> Self {
        self.compression = Compression::Zlib { quality };
        self
    }

    /// Enable the use of gzip compression algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[must_use]
    #[cfg(feature = "flate2")]
    pub fn gzip(mut self, quality: flate2::Compression) -> Self {
        self.compression = Compression::Gzip { quality };
        self
    }

    pub fn build(self) -> io::Result<Encoder<W>> {
        if !self.write.is_writeable() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'Write' implementation was not provided",
            ));
        }

        #[cfg(feature = "futures")]
        if self.write.is_async_writeable()
            && (self.write.is_async_seekable() != self.write.is_seekable())
        {
            if self.write.is_seekable() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'Seek' implementation was provided, but not 'AsyncSeek'",
                ));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'AsyncSeek' implementation was provided, but not 'Seek'",
                ));
            }
        }

        let jmptable_bufsize = (8 /* Timestamp (in millis) */ + 8 /* File pointer */ + 1/* Entry validity */)
            * self.jmptable_size
            + 8 /* File pointer to the next JMP chunk */
            + 1 /* Entry validity */;

        let mut enc = Encoder {
            write: self.write,
            compression: self.compression,
            jmptable_size: self.jmptable_size,
            jmptable_zeroes: if ZEROES.len() >= jmptable_bufsize {
                Cow::Borrowed(&ZEROES[0..jmptable_bufsize])
            } else {
                Cow::Owned(vec![0; jmptable_bufsize])
            },
            jmptable_remaining: 0,
            jmptable_at: JmpTableImpl::None,
            jmptable_at_ptr: 0,
            mod_act_buffer: vec![0; self.mod_buffer_size].into_boxed_slice(),
            mod_act_len: 0,

            epoch: Instant::now(),
        };

        swrite!(enc).write_all(b"MDR\0")?;
        swrite!(enc).flush()?;

        Ok(enc)
    }

    #[cfg(feature = "futures")]
    pub async fn build_async(self) -> io::Result<Encoder<W>> {
        use futures_util::AsyncWriteExt as _;

        if !self.write.is_async_writeable() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'AsyncWrite' implementation was not provided",
            ));
        }

        if self.write.is_writeable() && self.write.is_async_seekable() != self.write.is_seekable() {
            if self.write.is_seekable() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'Seek' implementation was provided, but not 'AsyncSeek'",
                ));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'AsyncSeek' implementation was provided, but not 'Seek'",
                ));
            }
        }

        let jmptable_bufsize = (8 /* Timestamp (in millis) */ + 8 /* File pointer */ + 1/* Entry validity */)
            * self.jmptable_size
            + 8 /* File pointer to the next JMP chunk */
            + 1 /* Entry validity */;

        let mut enc = Encoder {
            write: self.write,
            compression: self.compression,
            jmptable_size: self.jmptable_size,
            jmptable_zeroes: if ZEROES.len() >= jmptable_bufsize {
                Cow::Borrowed(&ZEROES[0..jmptable_bufsize])
            } else {
                Cow::Owned(vec![0; jmptable_bufsize])
            },
            jmptable_remaining: 0,
            jmptable_at: JmpTableImpl::None,
            jmptable_at_ptr: 0,
            jmptable_last_ts: 0,
            jmptable_waittime: self.jmptable_waittime,
            mod_act_buffer: vec![0; self.mod_buffer_size].into_boxed_slice(),
            mod_act_len: 0,

            epoch: Instant::now(),
        };

        awrite!(enc).write_all(b"MDR\0").await?;
        awrite!(enc).flush().await?;

        Ok(enc)
    }
}
impl<W: Write> Builder<W> {
    /// Make this [Encoder] writeable.
    ///
    /// This is required for the encoder to be operational in sync mode.
    #[must_use]
    pub fn writeable(mut self) -> Self {
        self.write.make_writeable();
        self
    }
}
impl<W: Seek> Builder<W> {
    /// Make this [Encoder] seekable.
    ///
    /// Will enable creation of JMP chunks for fast seeking.
    #[must_use]
    pub fn seekable(mut self) -> Self {
        self.write.make_seekable();
        self
    }
}
#[cfg(feature = "futures")]
impl<W: AsyncSeek> Builder<W> {
    /// Make this [Encoder] seekable.
    ///
    /// Will enable creation of JMP chunks for fast seeking.
    #[must_use]
    pub fn async_seekable(mut self) -> Self {
        self.write.make_async_seekable();
        self
    }
}
#[cfg(feature = "futures")]
impl<W: AsyncWrite> Builder<W> {
    /// Make this [Encoder] writeable.
    ///
    /// Will enable the use of async functions.
    #[must_use]
    pub fn async_writeable(mut self) -> Self {
        self.write.make_async_writeable();
        self
    }
}
impl<W: TryClone> Builder<W> {
    /// Make this [Encoder] try-cloneable.
    ///
    /// Will enable use of another file descriptor for writing into the current
    /// JMP chunk.
    #[must_use]
    pub fn try_cloneable(mut self) -> Self {
        self.write.make_try_cloneable();
        self
    }
}

enum JmpTableImpl<W> {
    None,
    Addr(u64),
    File(GenericIo<W>),
}

/// MDR encoder.
///
/// As of now it only supports writing new files, but not appending to
/// existing ones.
pub struct Encoder<W> {
    /// Instant at which file had been created.
    epoch: Instant,

    /// Compression to use for large chunks.
    compression: Compression,

    /// Amount of milliseconds to wait before creating a new jump table.
    jmptable_waittime: u64,
    /// Amount of entries in jump table.
    jmptable_size: usize,
    /// File pointer location of the previous jump table.
    ///
    /// `0` is a jump table was not yet created.
    jmptable_at_ptr: u64,
    /// Zeroes to add for jump table.
    jmptable_zeroes: Cow<'static, [u8]>,
    /// Remaining entries to be added.
    jmptable_remaining: usize,
    /// Last timestamp recorded in the newest jump table, or
    /// a timestamp of when a timestamp was created.
    jmptable_last_ts: u64,
    /// Location of the current JMP chunk.
    jmptable_at: JmpTableImpl<W>,

    /// Buffer for modifications.
    mod_act_buffer: Box<[u8]>,
    /// Length of data used in [Self::mod_act_buffer].
    mod_act_len: usize,

    /// Writing destination.
    write: GenericIo<W>,
}
impl<W> Encoder<W> {
    /// Create a new [Builder].
    #[must_use]
    #[inline(always)]
    pub fn builder(write: W) -> Builder<W> {
        Builder::new(write)
    }

    /// Create a streaming [Encoder].
    ///
    /// This is equivalent to `Encoder::builder(write).build()`
    #[inline(always)]
    pub fn streaming(write: W) -> io::Result<Self> {
        Encoder::builder(write).build()
    }

    /// Create a streaming [Encoder].
    ///
    /// This is equivalent to `Encoder::builder(write).build()`
    #[inline(always)]
    #[cfg(feature = "futures")]
    pub fn streaming_async(write: W) -> io::Result<Self>
    where
        W: AsyncWrite,
    {
        Encoder::builder(write).async_writeable().build()
    }

    /// Check if this encoder supports synchronous writing.
    #[must_use]
    #[inline(always)]
    pub const fn is_sync(&self) -> bool {
        self.write.is_writeable()
    }

    /// Check if this encoder supports synchronous writing.
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "futures")]
    pub const fn is_async(&self) -> bool {
        self.write.is_async_writeable()
    }

    /// Check availability and consistency of features.
    fn expect_features(&self, r#async: bool) -> io::Result<()> {
        if !r#async && !self.is_sync() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "backend does not support sync writing",
            ));
        }

        #[cfg(feature = "futures")]
        if r#async && !self.is_async() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "backend does not support async writing",
            ));
        }

        #[cfg(feature = "futures")]
        if self.is_async()
            && self.is_sync()
            && (self.write.is_async_seekable() != self.write.is_seekable())
        {
            if self.write.is_seekable() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'Seek' implementation was provided, but not 'AsyncSeek'",
                ));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "'AsyncSeek' implementation was provided, but not 'Seek'",
                ));
            }
        }

        #[cfg(not(feature = "futures"))]
        {
            _ = r#async;
        }

        Ok(())
    }

    /// Whether jump table can be created.
    ///
    /// Since both sync and async require matching seek features, it's safe to
    /// OR both.
    const fn can_make_jmpt(&self) -> bool {
        self.write.is_seekable() || {
            #[cfg(feature = "futures")]
            {
                self.write.is_async_seekable()
            }
            #[cfg(not(feature = "futures"))]
            {
                false
            }
        }
    }

    /// Write a chunk.
    ///
    /// **Note**: This will automatically create a jump table entry, or allocate a
    ///           jump table if one doesn't exist or the existing one is full.
    ///
    /// ## Params
    /// **timestamp** - milliseconds since the start of recording
    fn write_chunk(&mut self, name: ChunkKind, timestamp: u64, data: &[u8]) -> io::Result<()> {
        self.expect_features(false)?;

        if name == ChunkKind::Jmp {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "jump tables must never be created manually",
            ));
        }

        if data.len() > i32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too much data to contain in one chunk",
            ));
        }

        if self.can_make_jmpt() {
            let this_pos = sseek!(self).stream_position()?;
            let prev_jmpt = self.jmptable_at_ptr;

            match &self.jmptable_at {
                JmpTableImpl::None => {
                    self.jmptable_at_ptr = this_pos;
                    self.jmptable_last_ts = timestamp;

                    let mut header = [0; 1 + 8 + 4];
                    header[0] = ChunkKind::Jmp.ordinal();
                    header[1..][..8].copy_from_slice(&timestamp.to_le_bytes());
                    header[1 + 8..][..4].copy_from_slice(&(data.len() as u32).to_le_bytes());

                    if let Some(new_io) = self.write.try_clone() {
                        self.jmptable_at = JmpTableImpl::File(new_io?);
                    } else {
                        self.jmptable_at = JmpTableImpl::Addr(self.jmptable_at_ptr);
                    }
                }
                JmpTableImpl::Addr(addr) => {
                    if self.jmptable_last_ts - timestamp > self.jmptable_zeroes
                },
                JmpTableImpl::File(generic_io) => {}
            }
        }

        let mut header = [0; 1 + 8 + 4];
        header[0] = name.ordinal();
        header[1..][..8].copy_from_slice(&timestamp.to_le_bytes());
        header[1 + 8..][..4].copy_from_slice(&(data.len() as u32).to_le_bytes());

        swrite!(self).write_all(&header)?;
        swrite!(self).write_all(data)?;
        swrite!(self).write_all(&(data.len() as u32).to_le_bytes())?;
        swrite!(self).write_all(&self.jmptable_at_ptr.to_le_bytes())?;

        Ok(())
    }

    /// Write a chunk.
    ///
    /// **Note**: This will automatically create a jump table entry, or allocate a
    ///           jump table if one doesn't exist or the existing one is full.
    ///
    /// ## Params
    /// **timestamp** - milliseconds since the start of recording
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, making the file unreadable.
    #[cfg(feature = "futures")]
    async fn write_chunk_async(
        &mut self,
        name: ChunkKind,
        timestamp: u64,
        data: &[u8],
    ) -> io::Result<()> {
        use futures_util::AsyncWriteExt as _;

        self.expect_features(true)?;

        if name == ChunkKind::Jmp {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "jump tables must never be created manually",
            ));
        }

        if data.len() > i32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too much data to contain in one chunk",
            ));
        }

        let mut header = [0; 1 + 8 + 4];
        header[0] = name.ordinal();
        header[1..][..8].copy_from_slice(&timestamp.to_le_bytes());
        header[1 + 8..][..4].copy_from_slice(&(data.len() as u32).to_le_bytes());

        awrite!(self).write_all(&header).await?;
        awrite!(self).write_all(data).await?;
        awrite!(self)
            .write_all(&(data.len() as u32).to_le_bytes())
            .await?;

        Ok(())
    }

    /// Flush all written data.
    ///
    /// Simply triggers the underlying [Write::flush].
    pub fn flush(&mut self) -> io::Result<()> {
        self.expect_features(false)?;
        swrite!(self).flush()
    }

    /// Flush all written data.
    ///
    /// Simply triggers the underlying [AsyncWriteExt::flush].
    #[cfg(feature = "futures")]
    pub async fn flush_async(&mut self) -> io::Result<()> {
        use futures_util::AsyncWriteExt as _;

        self.expect_features(true)?;
        awrite!(self).flush().await
    }
}
