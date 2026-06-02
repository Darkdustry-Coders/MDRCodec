//! Properly typed encoder/decoders with a guaranteed asynchronous API.

use std::io;

use futures_io::{AsyncWrite, AsyncSeek};

use crate::{enc::Encoder, opt::Compression};

/// An asynchronous streaming encoder.
pub struct AsyncStreamingEncoder<W> {
    inner: Encoder<W>,
}
impl<W: AsyncWrite> AsyncStreamingEncoder<W> {
    /// Create a new [AsyncStreamingEncoder] instance.
    ///
    /// ## Cancellation safety
    ///
    /// Cancellation safety of this method depends on that of the [AsyncWrite]
    /// implementation.
    pub async fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .async_writeable()
            .compression(compression)
            .build_async().await?;
        Ok(Self { inner: encoder })
    }

    /// Flush the buffers.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush_async().await
    }
}

/// An asynchronous streaming encoder.
pub struct AsyncSeekingEncoder<W> {
    inner: Encoder<W>,
}
impl<W: AsyncWrite + AsyncSeek> AsyncSeekingEncoder<W> {
    /// Create a new [AsyncSeekingEncoder] instance.
    ///
    /// ## Cancellation safety
    ///
    /// Cancellation safety of this method depends on that of the [AsyncWrite]
    /// implementation.
    pub async fn new(write: W, compression: Compression) -> io::Result<Self> {
        let encoder = Encoder::builder(write)
            .async_writeable()
            .async_seekable()
            .compression(compression)
            .build_async().await?;
        Ok(Self { inner: encoder })
    }

    /// Flush the buffers.
    ///
    /// If there are unsaved modifications, a new MOD chunk will be created,
    /// alongside all other required chunks.
    ///
    /// ## Cancellation safety
    ///
    /// This method is *not* cancel-safe. Cancelling this method may
    /// result in broken chunks, as well as corrupting the internal
    /// buffer, making the file unreadable.
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush_async().await
    }
}
