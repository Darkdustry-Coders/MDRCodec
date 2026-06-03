//! Internal encoder implementation.
//!
//! The actual underlying encoder is runtime-typed. If you want to
//! have proper typing, consider using [crate::sync] or [crate::future].

use std::{
    borrow::Cow, io::{self, Cursor, Seek, Write}, marker::PhantomData, mem::transmute, time::{Duration, Instant}
};

#[cfg(feature = "futures")]
use futures_io::{AsyncSeek, AsyncWrite};
#[cfg(feature = "futures")]
use futures_util::AsyncSeekExt;

use crate::{
    data::{ChangeKind, ChunkKind, TileAccess, WorldAccess},
    io::{GenericIo, TryClone, WriteExactly, WriteExt},
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

macro_rules! uswrite {
    ($self:expr) => {
        unsafe { $self.write.writeable().unwrap_unchecked() }
    };
}
macro_rules! usseek {
    ($self:expr) => {
        unsafe { $self.write.seekable().unwrap_unchecked() }
    };
}

#[cfg(feature = "futures")]
macro_rules! uawrite {
    ($self:expr) => {
        unsafe { $self.write.async_writeable().unwrap_unchecked() }
    };
}
#[cfg(feature = "futures")]
macro_rules! uaseek {
    ($self:expr) => {
        unsafe { $self.write.async_seekable().unwrap_unchecked() }
    };
}

/// A lot of zeroes to write the entire JMP table in one go.
///
/// If the configured JMP table needs less zeroes than this buffer provides,
/// it will not allocate a new buffer.
static ZEROES: &[u8] = &[0; (1 /* Entry chunk type */ +
                             8 /* Timestamp (in millis) */ +
                             8 /* File pointer */ +
                             1 /* Entry validity */)
   * 1023
   + 8 /* File pointer to the next JMP chunk */ + 1 /* Entry validity */];

/// Encoder builder.
pub struct Builder<W> {
    write: GenericIo<W>,
    snapshot_duration: u64,
    jmptable_size: usize,
    mod_buffer_size: usize,
    compression: Compression,

    _phantom: PhantomData<W>,
}
impl<W> Builder<W> {
    #[must_use]
    pub fn new(write: W) -> Self {
        Self {
            write: GenericIo::new_empty(write),
            snapshot_duration: 120000,
            jmptable_size: 1024,
            mod_buffer_size: 1024 * 64,
            compression: Compression::None,

            _phantom: PhantomData,
        }
    }

    /// Specify how long will it take until a new MAP chunk is created.
    ///
    /// If [Duration::ZERO], only a MAP chunk at the start will be created.
    ///
    /// Only takes effect if seeking is enabled as otherwise MAP chunks aren't
    /// necessary.
    #[must_use]
    pub fn snapshot_duration(mut self, duration: Duration) -> Self {
        if duration.is_zero() {
            self.snapshot_duration = 0;
        } else {
            self.snapshot_duration = duration.as_millis().min(1000) as u64;
        }
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
        self.jmptable_size = size.max(1).min(i32::MAX as usize);
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

        let jmptable_bufsize = (1 /* Entry chunk type */ +
                                       8 /* Timestamp (in millis) */ +
                                       8 /* File pointer */ +
                                       1 /* Entry validity */)
            * (self.jmptable_size - 1)
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

            mod_act_buffer: vec![0; self.mod_buffer_size].into_boxed_slice(),
            mod_act_len: 0,
            snapshot_rate: 0,
            mod_start_ts: self.snapshot_duration,

            epoch: Instant::now(),
        };

        let mut write = swrite!(enc);
        write.write_all(b"MDR\0")?;
        write.write_u16_le(1)?;
        self.compression.write(&mut write)?;
        write.write_all(&[0; 8 + 8 + 4 + 1])?;
        write.flush()?;

        Ok(enc)
    }

    #[cfg(feature = "futures")]
    pub async fn build_async(self) -> io::Result<Encoder<W>> {
        use futures_util::AsyncWriteExt as _;

        use crate::io::AsyncWriteExt;

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

        let jmptable_bufsize = (1 /* Entry chunk type */ +
                                       8 /* Timestamp (in millis) */ +
                                       8 /* File pointer */ +
                                       1 /* Entry validity */)
            * (self.jmptable_size - 1)
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

            mod_act_buffer: vec![0; self.mod_buffer_size].into_boxed_slice(),
            mod_act_len: 0,
            snapshot_rate: 0,
            mod_start_ts: self.snapshot_duration,

            epoch: Instant::now(),
        };

        let mut write = awrite!(enc);
        write.write_all(b"MDR\0").await?;
        write.write_u16_le(1).await?;
        self.compression.write_async(&mut write).await?;
        write.write_all(&[0; 8 + 8 + 4 + 1]).await?;
        write.flush().await?;

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
    ///
    /// If `0`, buffer is not being used.
    mod_act_len: usize,
    /// Starting timestamp for the current modification buffer.
    mod_start_ts: u64,

    /// Maximum duration until the buffer is submitted.
    snapshot_rate: u64,

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

    pub const fn snapshot_rate(&self) -> Duration {
        Duration::from_millis(self.snapshot_rate)
    }

    #[must_use]
    fn timestamp(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
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

    // TODO: Implement a macro to simplify sync/async impls. This is horrific.

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

            match &mut self.jmptable_at {
                JmpTableImpl::None => {
                    let mut write = uswrite!(self);
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 8 + 4}>::new();
                        write.write_u8(ChunkKind::Jmp.ordinal())?;
                        write.write_u64_le(timestamp)?;
                        write.write_u32_le(self.jmptable_zeroes.len() as u32)?;
                        write.finalize()?
                    })?;
                    write.write_all(&self.jmptable_zeroes)?;
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 4 + 8 + 8}>::new();
                        write.write_u8(ChunkKind::Jmp.ordinal())?;
                        write.write_u32_le(self.jmptable_zeroes.len() as u32)?;
                        write.write_u64_le(timestamp)?;
                        write.write_u64_le(self.jmptable_at_ptr)?;
                        write.finalize()?
                    })?;

                    self.jmptable_at_ptr = this_pos;
                    self.jmptable_last_ts = timestamp;
                    self.jmptable_remaining = self.jmptable_size;

                    if let Some(new_io) = self.write.try_clone() {
                        self.jmptable_at = JmpTableImpl::File(new_io?);
                    } else {
                        self.jmptable_at = JmpTableImpl::Addr(self.jmptable_at_ptr + 1 + 8 + 4);
                    }
                },
                JmpTableImpl::Addr(addr) => {
                    self.jmptable_remaining -= 1;
                    usseek!(self).seek(io::SeekFrom::Start(*addr))?;
                    let mut write = swrite!(self);
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 8 + 8}>::new();
                        write.write_u8(name.ordinal())?;
                        write.write_u64_le(timestamp)?;
                        write.write_u64_le(this_pos)?;
                        write.finalize()?
                    })?;
                    *addr += 1 + 8 + 8;
                    usseek!(self).seek(io::SeekFrom::Start(this_pos))?;
                },
                JmpTableImpl::File(io) => {
                    self.jmptable_remaining -= 1;
                    let mut write = io.writeable().unwrap();
                    write.write_u8(name.ordinal())?;
                    write.write_u64_le(timestamp)?;
                    write.write_u64_le(this_pos)?;
                    write.flush()?;
                }
            }
        }

        let buf = if matches!(&self.compression, Compression::None) {
            Cow::Borrowed(data)
        } else {
            let mut buf = vec![];
            self.compression.write_data(Cursor::new(&mut buf), data)?;
            Cow::Owned(buf)
        };

        let mut write = swrite!(self);
        write.write_all(&{
            let mut write = WriteExactly::<{1 + 8 + 4}>::new();
            write.write_u8(name.ordinal())?;
            write.write_u64_le(timestamp)?;
            write.write_u32_le(buf.len() as u32)?;
            write.finalize()?
        })?;
        write.write_all(&buf)?;
        write.write_all(&{
            let mut write = WriteExactly::<{1 + 4 + 8 + 8}>::new();
            write.write_u8(name.ordinal())?;
            write.write_u32_le(buf.len() as u32)?;
            write.write_u64_le(timestamp)?;
            write.write_u64_le(self.jmptable_at_ptr)?;
            write.finalize()?
        })?;

        match &mut self.jmptable_at {
            JmpTableImpl::None => (),
            JmpTableImpl::Addr(addr) => {
                let this_pos = usseek!(self).stream_position()?;
                usseek!(self).seek(io::SeekFrom::Start(*addr))?;
                uswrite!(self).write_u8(1)?;
                usseek!(self).seek(io::SeekFrom::Start(this_pos))?;
                *addr += 1;
            },
            JmpTableImpl::File(io) => {
                let mut write = unsafe { io.writeable().unwrap_unchecked() };
                write.write_u8(1)?;
                write.flush()?;
            },
        }
        if self.jmptable_remaining == 0 { self.jmptable_at = JmpTableImpl::None; }

        uswrite!(self).flush()?;

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
    async fn write_chunk_async(&mut self, name: ChunkKind, timestamp: u64, data: &[u8]) -> io::Result<()> {
        use futures_util::AsyncWriteExt as _;
        use crate::io::AsyncWriteExt as _;

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

        if self.can_make_jmpt() {
            let this_pos = aseek!(self).stream_position().await?;

            match &mut self.jmptable_at {
                JmpTableImpl::None => {
                    let mut write = uawrite!(self);
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 8 + 4}>::new();
                        write.write_u8(ChunkKind::Jmp.ordinal())?;
                        write.write_u64_le(timestamp)?;
                        write.write_u32_le(self.jmptable_zeroes.len() as u32)?;
                        write.finalize()?
                    }).await?;
                    write.write_all(&self.jmptable_zeroes).await?;
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 4 + 8 + 8}>::new();
                        write.write_u8(ChunkKind::Jmp.ordinal())?;
                        write.write_u32_le(self.jmptable_zeroes.len() as u32)?;
                        write.write_u64_le(timestamp)?;
                        write.write_u64_le(self.jmptable_at_ptr)?;
                        write.finalize()?
                    }).await?;

                    self.jmptable_at_ptr = this_pos;
                    self.jmptable_last_ts = timestamp;
                    self.jmptable_remaining = self.jmptable_size;

                    if let Some(new_io) = self.write.try_clone() {
                        self.jmptable_at = JmpTableImpl::File(new_io?);
                    } else {
                        self.jmptable_at = JmpTableImpl::Addr(self.jmptable_at_ptr + 1 + 8 + 4);
                    }
                },
                JmpTableImpl::Addr(addr) => {
                    self.jmptable_remaining -= 1;
                    uaseek!(self).seek(io::SeekFrom::Start(*addr)).await?;
                    let mut write = awrite!(self);
                    write.write_all(&{
                        let mut write = WriteExactly::<{1 + 8 + 8}>::new();
                        write.write_u8(name.ordinal())?;
                        write.write_u64_le(timestamp)?;
                        write.write_u64_le(this_pos)?;
                        write.finalize()?
                    }).await?;
                    *addr += 1 + 8 + 8;
                    uaseek!(self).seek(io::SeekFrom::Start(this_pos)).await?;
                },
                JmpTableImpl::File(io) => {
                    self.jmptable_remaining -= 1;
                    let mut write = io.async_writeable().unwrap();
                    write.write_u8(name.ordinal()).await?;
                    write.write_u64_le(timestamp).await?;
                    write.write_u64_le(this_pos).await?;
                    write.flush().await?;
                }
            }
        }

        let buf = if matches!(&self.compression, Compression::None) {
            Cow::Borrowed(data)
        } else {
            let mut buf = vec![];
            self.compression.write_data(Cursor::new(&mut buf), data)?;
            Cow::Owned(buf)
        };

        let mut write = awrite!(self);
        write.write_all(&{
            let mut write = WriteExactly::<{1 + 8 + 4}>::new();
            write.write_u8(name.ordinal())?;
            write.write_u64_le(timestamp)?;
            write.write_u32_le(buf.len() as u32)?;
            write.finalize()?
        }).await?;
        write.write_all(&buf).await?;
        write.write_all(&{
            let mut write = WriteExactly::<{1 + 4 + 8 + 8}>::new();
            write.write_u8(name.ordinal())?;
            write.write_u32_le(buf.len() as u32)?;
            write.write_u64_le(timestamp)?;
            write.write_u64_le(self.jmptable_at_ptr)?;
            write.finalize()?
        }).await?;

        match &mut self.jmptable_at {
            JmpTableImpl::None => (),
            JmpTableImpl::Addr(addr) => {
                let this_pos = usseek!(self).stream_position()?;
                uaseek!(self).seek(io::SeekFrom::Start(*addr)).await?;
                uawrite!(self).write_u8(1).await?;
                uaseek!(self).seek(io::SeekFrom::Start(this_pos)).await?;
                *addr += 1;
            },
            JmpTableImpl::File(io) => {
                let mut write = unsafe { io.async_writeable().unwrap_unchecked() };
                write.write_u8(1).await?;
                write.flush().await?;
            },
        }
        if self.jmptable_remaining == 0 { self.jmptable_at = JmpTableImpl::None; }

        uawrite!(self).flush().await?;

        Ok(())
    }

    /// Write MAP chunk.
    pub fn write_map<M: WorldAccess>(&mut self, map: M) -> io::Result<()> {
        self.flush_mod()?;

        let mut buf = Cursor::new(vec![]);
        buf.write_u32_le(map.width())?;
        buf.write_u32_le(map.height())?;
        for y in 0..map.height() { for x in 0..map.width() {
            let tile = map.tile(x, y).unwrap();
            buf.write_u16_le(tile.block())?;
            buf.write_u16_le(tile.floor())?;
            buf.write_u16_le(tile.overlay())?;
            buf.write_u8(tile.data_block())?;
            buf.write_u8(tile.data_floor())?;
            buf.write_u8(tile.data_overlay())?;
            buf.write_u32_le(tile.data_extra())?;
            buf.write_u8(0)?;
        } }

        self.write_chunk(ChunkKind::Map, self.timestamp(), &buf.into_inner())?;

        Ok(())
    }

    /// Write MAP chunk.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    pub async fn write_map_async<M: WorldAccess>(&mut self, map: M) -> io::Result<()> {
        self.flush_mod_async().await?;

        let mut buf = Cursor::new(vec![]);
        buf.write_u32_le(map.width())?;
        buf.write_u32_le(map.height())?;
        for y in 0..map.height() { for x in 0..map.width() {
            let tile = map.tile(x, y).unwrap();
            buf.write_u16_le(tile.block())?;
            buf.write_u16_le(tile.floor())?;
            buf.write_u16_le(tile.overlay())?;
            buf.write_u8(tile.data_block())?;
            buf.write_u8(tile.data_floor())?;
            buf.write_u8(tile.data_overlay())?;
            buf.write_u32_le(tile.data_extra())?;
            buf.write_u8(0)?;
        } }

        self.write_chunk_async(ChunkKind::Map, self.timestamp(), &buf.into_inner()).await?;

        Ok(())
    }

    /// Write MAP chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid MAP chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_map_raw(&mut self, map: &[u8]) -> io::Result<()> {
        self.flush_mod()?;

        self.write_chunk(ChunkKind::Map, self.timestamp(), map)?;

        Ok(())
    }

    /// Write MAP chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid MAP chunk body.
    ///
    /// If not, the file may become unparseable.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    pub async unsafe fn write_map_raw_async(&mut self, map: &[u8]) -> io::Result<()> {
        self.flush_mod_async().await?;

        self.write_chunk_async(ChunkKind::Map, self.timestamp(), map).await?;

        Ok(())
    }

    /// Write ID chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    pub unsafe fn write_id_raw(&mut self, map: &[u8]) -> io::Result<()> {
        self.flush_mod()?;

        self.write_chunk(ChunkKind::Id, self.timestamp(), map)?;

        Ok(())
    }

    /// Write ID chunk using raw data.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    pub async unsafe fn write_id_raw_async(&mut self, map: &[u8]) -> io::Result<()> {
        self.flush_mod_async().await?;

        self.write_chunk_async(ChunkKind::Id, self.timestamp(), map).await?;

        Ok(())
    }

    /// Try write change into the buffer.
    ///
    /// Returns `true` if writing succeeded, otherwise `false`.
    fn write_change_buf(&mut self, change: &ChangeKind) -> bool {
        let offset = ((Instant::now() - self.epoch).as_millis() as u64 - self.mod_start_ts) as u32;
        match change {
            ChangeKind::UnitMoved { unit_id, x, y } => {
                let mut write = WriteExactly::<{ 1 + 4 + 4 + 4 + 4 }>::new();
                write.write_u32_le(offset).unwrap();
                write.write_u8(1).unwrap();
                write.write_i32_le(*unit_id).unwrap();
                write.write_f32_le(*x).unwrap();
                write.write_f32_le(*y).unwrap();
                let buf = write.finalize().unwrap();
                if self.mod_act_buffer.len() - self.mod_act_len < buf.len() { return false; }
                self.mod_act_buffer[self.mod_act_len..][..buf.len()].copy_from_slice(&buf);
                self.mod_act_len += buf.len();
            },
            ChangeKind::UnitRotation { unit_id, rot } => {
                let mut write = WriteExactly::<{ 1 + 4 + 4 + 1 }>::new();
                write.write_u32_le(offset).unwrap();
                write.write_u8(2).unwrap();
                write.write_i32_le(*unit_id).unwrap();
                write.write_u8(*rot).unwrap();
                let buf = write.finalize().unwrap();
                if self.mod_act_buffer.len() - self.mod_act_len < buf.len() { return false; }
                self.mod_act_buffer[self.mod_act_len..][..buf.len()].copy_from_slice(&buf);
                self.mod_act_len += buf.len();
            },
            ChangeKind::UnitDead { unit_id } => {
                let mut write = WriteExactly::<{ 1 + 4 + 4 }>::new();
                write.write_u32_le(offset).unwrap();
                write.write_u8(3).unwrap();
                write.write_i32_le(*unit_id).unwrap();
                let buf = write.finalize().unwrap();
                if self.mod_act_buffer.len() - self.mod_act_len < buf.len() { return false; }
                self.mod_act_buffer[self.mod_act_len..][..buf.len()].copy_from_slice(&buf);
                self.mod_act_len += buf.len();
            },
            ChangeKind::UnitDespawn { unit_id } => {
                let mut write = WriteExactly::<{ 1 + 4 + 4 }>::new();
                write.write_u32_le(offset).unwrap();
                write.write_u8(4).unwrap();
                write.write_i32_le(*unit_id).unwrap();
                let buf = write.finalize().unwrap();
                if self.mod_act_buffer.len() - self.mod_act_len < buf.len() { return false; }
                self.mod_act_buffer[self.mod_act_len..][..buf.len()].copy_from_slice(&buf);
                self.mod_act_len += buf.len();
            },
        }
        true
    }

    /// Append a change.
    ///
    /// This will create a mod chunk once another chunk will need to be
    /// written, a write timeout has expired, or the writer is flushed,
    /// or the modifications buffer is full.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    pub fn write_change(&mut self, change: &ChangeKind) -> io::Result<()> {
        if self.mod_act_len == 0 {
            self.mod_start_ts = (Instant::now() - self.epoch).as_millis() as u64;
        }

        if !self.write_change_buf(change) {
            self.flush_mod()?;
            if !self.write_change_buf(change) {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "modifications buffer too short"));
            }
        }

        Ok(())
    }

    /// Append a change.
    ///
    /// This will create a mod chunk once another chunk will need to be
    /// written, a write timeout has expired, or the writer is flushed,
    /// or the modifications buffer is full.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the passed data is a valid ID chunk body.
    ///
    /// If not, the file may become unparseable.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    pub async fn write_change_async(&mut self, change: &ChangeKind) -> io::Result<()> {
        if self.mod_act_len == 0 {
            self.mod_start_ts = (Instant::now() - self.epoch).as_millis() as u64;
        }

        if !self.write_change_buf(change) {
            self.flush_mod_async().await?;
            if !self.write_change_buf(change) {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "modifications buffer too short"));
            }
        }

        Ok(())
    }

    /// Flush modifications data.
    fn flush_mod(&mut self) -> io::Result<()> {
        self.expect_features(false)?;

        if self.mod_act_len > 0 {
            // Safety: `write_chunk` never modifies this buffer, so it's safe.
            let buffer: &'static [u8] = unsafe { transmute(&self.mod_act_buffer[0..self.mod_act_len]) };
            self.write_chunk(ChunkKind::Mod, self.mod_start_ts, buffer)?;
            self.mod_act_len = 0;
        }

        Ok(())
    }

    /// Flush modifications data.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    async fn flush_mod_async(&mut self) -> io::Result<()> {
        self.expect_features(true)?;

        if self.mod_act_len > 0 {
            // Safety: `write_chunk_async` never modifies this buffer, so it's safe.
            //         While it's possible that this method gets cancelled and another write
            //         into this buffer may occur and do some unexpected things, that's entirely
            //         on dev's fault.
            let buffer: &'static [u8] = unsafe { transmute(&self.mod_act_buffer[0..self.mod_act_len]) };
            self.write_chunk_async(ChunkKind::Mod, self.mod_start_ts, buffer).await?;
            self.mod_act_len = 0;
        }

        Ok(())
    }

    /// Flush all written data.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    pub fn flush(&mut self) -> io::Result<()> {
        self.flush_mod()?;
        swrite!(self).flush()
    }

    /// Flush all written data.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    #[cfg(feature = "futures")]
    pub async fn flush_async(&mut self) -> io::Result<()> {
        use futures_util::AsyncWriteExt as _;

        self.flush_mod_async().await?;
        awrite!(self).flush().await
    }
}
