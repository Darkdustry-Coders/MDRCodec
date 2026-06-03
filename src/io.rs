//! Generic IO / Multiplatform file handles.
//!
//! ## Migration notice
//!
//! This may get merged into `libcommons` in the future. After
//! the merge this module will be replaced with a redirect.

#[cfg(feature = "futures")]
use std::pin::pin;
use std::{
    fs::File,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream, UdpSocket},
};

#[cfg(target_os = "windows")]
use std::os::windows::io::RawHandle as Handle;
#[cfg(unix)]
use std::os::{
    fd::{AsRawFd, IntoRawFd},
    unix::net::{UnixDatagram, UnixListener, UnixStream},
};

#[cfg(feature = "futures")]
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};

/// Mark trait as being safe to instantiate via zeroing the memory.
pub unsafe trait Zeroed {}

#[inline(always)]
pub const fn zeroed<T: Zeroed>() -> T { unsafe { std::mem::zeroed() } }
pub fn zeroed_vec<T: Zeroed>(len: usize) -> Vec<T> { unsafe {
    let mut vec = Vec::with_capacity(len);
    vec.set_len(len);
    for x in vec.iter_mut() {
        let x = x as *mut T;
        x.write_bytes(0, size_of::<T>());
    }
    vec
} }

/// A trait to abstract over various implementations of `try_clone`.
pub trait TryClone: Sized {
    /// Make a clone of this object.
    fn try_clone(&self) -> io::Result<Self>;
}
impl TryClone for File {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
impl TryClone for TcpStream {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
impl TryClone for TcpListener {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
impl TryClone for UdpSocket {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
#[cfg(unix)]
impl TryClone for UnixStream {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
#[cfg(unix)]
impl TryClone for UnixDatagram {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
#[cfg(unix)]
impl TryClone for UnixListener {
    fn try_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}

/// Convert an object into a raw handle for passing it over FFI.
pub trait AsRawHandle {
    /// Object's raw handle.
    ///
    /// Handles are not automatically closed with RAII, so you must do
    /// so manually.
    type Handle;

    /// Obtain a raw handle.
    ///
    /// ## Safety
    ///
    /// Due to it potentially causing issues with `close()`, this method
    /// is marked as unsafe.
    unsafe fn as_raw(&self) -> Self::Handle;

    /// Convert this object into a raw handle.
    ///
    /// Handles are not automatically closed with RAII, so you must do
    /// so manually.
    fn into_raw(self) -> Self::Handle;
}

/// Raw file handle.
///
/// This type wraps over whatever type is preferred on this system.
///
/// For UNIX systems this type is [i32]. For Windows it's
/// [::std::os::windows::io::RawHandle]. This struct is guaranteed
/// to be safe to transmute as long as you're using the correct
/// type.
///
/// If you're not sure, use the [RawFileHandle::into_inner] method.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RawFileHandle(#[cfg(unix)] i32, #[cfg(target_os = "windows")] Handle);
impl RawFileHandle {
    /// Get the actual file handle.
    #[cfg(unix)]
    pub fn into_inner(self) -> i32 {
        self.0
    }

    /// Construct a [File] out of this file handle.
    ///
    /// ## Safety
    ///
    /// Since this method could potentially create multiple
    /// [File]s referencing the same file descriptor, this method
    /// is marked as unsafe.
    #[cfg(unix)]
    pub unsafe fn into_file(self) -> File {
        use std::os::fd::FromRawFd;

        unsafe { File::from_raw_fd(self.0) }
    }

    /// Get the actual file handle.
    #[cfg(target_os = "windows")]
    pub fn into_inner(self) -> Handle {
        self.0
    }

    /// Construct a [File] out of this file handle.
    ///
    /// Since this method could potentially create multiple
    /// [File]s referencing the same file descriptor, this method
    /// is marked as unsafe.
    #[cfg(target_os = "windows")]
    pub unsafe fn into_file(self) -> File {
        use std::os::io::FromRawHandle;

        unsafe { File::from_raw_handle(self.0) }
    }
}

#[cfg(unix)]
impl AsRawHandle for File {
    type Handle = RawFileHandle;

    unsafe fn as_raw(&self) -> Self::Handle {
        RawFileHandle(self.as_raw_fd())
    }
    fn into_raw(self) -> Self::Handle {
        RawFileHandle(self.into_raw_fd())
    }
}

#[cfg(target_os = "windows")]
impl AsRawHandle for File {
    type Handle = RawFileHandle;

    unsafe fn as_raw(&self) -> Self::Handle {
        RawFileHandle(std::os::windows::io::AsRawHandle::as_raw_handle(self))
    }
    fn into_raw(self) -> Self::Handle {
        RawFileHandle(std::os::windows::io::IntoRawHandle::into_raw_handle(self))
    }
}

/// Write exactly N bytes.
///
/// Used to make IO more performant without making it a whole
/// lot worse.
///
/// The internal buffer is stored on stack.
pub struct WriteExactly<const LEN: usize> {
    write: Cursor<[u8; LEN]>,
}
impl<const LEN: usize> WriteExactly<LEN> {
    /// Create new [WriteExactly].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            write: Cursor::new([0; LEN]),
        }
    }

    /// Finalize this [WriteExact].
    ///
    /// Returns [Err] if buffer wasn't fully filled.
    pub fn finalize(self) -> io::Result<[u8; LEN]> {
        if self.write.position() as usize != LEN {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill the whole buffer"));
        }

        Ok(self.write.into_inner())
    }
}
impl<const LEN: usize> Write for WriteExactly<LEN> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write.write(buf)
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

struct SeekVTable<O> {
    pub seek: fn(&mut O, pos: SeekFrom) -> io::Result<u64>,
    pub rewind: fn(&mut O) -> io::Result<()>,
    pub seek_relative: fn(&mut O, offset: i64) -> io::Result<()>,
    pub stream_position: fn(&mut O) -> io::Result<u64>,
}
impl<O> Clone for SeekVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<O> Copy for SeekVTable<O> {}
impl<O: Seek> SeekVTable<O> {
    pub const fn new() -> Self {
        Self {
            seek: O::seek,
            rewind: O::rewind,
            seek_relative: O::seek_relative,
            stream_position: O::stream_position,
        }
    }
}

/// A [Seek] object.
pub struct Seekable<'a, W> {
    obj: &'a mut W,
    vtable: &'a SeekVTable<W>,
}
impl<'a, W> Seek for Seekable<'a, W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        (self.vtable.seek)(self.obj, pos)
    }

    fn rewind(&mut self) -> io::Result<()> {
        (self.vtable.rewind)(self.obj)
    }

    fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        (self.vtable.seek_relative)(self.obj, offset)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        (self.vtable.stream_position)(self.obj)
    }
}

struct WriteVTable<O> {
    write: fn(this: &mut O, buf: &[u8]) -> io::Result<usize>,
    write_all: fn(this: &mut O, buf: &[u8]) -> io::Result<()>,
    flush: fn(this: &mut O) -> io::Result<()>,
    write_fmt: fn(this: &mut O, args: std::fmt::Arguments<'_>) -> io::Result<()>,
    write_vectored: fn(this: &mut O, bufs: &[io::IoSlice<'_>]) -> io::Result<usize>,
}
impl<O> Clone for WriteVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<O> Copy for WriteVTable<O> {}
impl<O: Write> WriteVTable<O> {
    pub const fn new() -> Self {
        Self {
            write: O::write,
            write_all: O::write_all,
            flush: O::flush,
            write_fmt: O::write_fmt,
            write_vectored: O::write_vectored,
        }
    }
}

/// A [Write] object.
pub struct Writeable<'a, W> {
    obj: &'a mut W,
    vtable: &'a WriteVTable<W>,
}
impl<'a, W> Write for Writeable<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (self.vtable.write)(self.obj, buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        (self.vtable.write_all)(self.obj, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (self.vtable.flush)(self.obj)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> io::Result<()> {
        (self.vtable.write_fmt)(self.obj, args)
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        (self.vtable.write_vectored)(self.obj, bufs)
    }
}

struct ReadVTable<O> {
    read: fn(this: &mut O, buf: &mut [u8]) -> io::Result<usize>,
    read_exact: fn(this: &mut O, buf: &mut [u8]) -> io::Result<()>,
    read_to_string: fn(this: &mut O, string: &mut String) -> io::Result<usize>,
    read_vectored: fn(this: &mut O, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize>,
}
impl<O> Clone for ReadVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<O> Copy for ReadVTable<O> {}
impl<O: Read> ReadVTable<O> {
    pub const fn new() -> Self {
        Self {
            read: O::read,
            read_exact: O::read_exact,
            read_to_string: O::read_to_string,
            read_vectored: O::read_vectored,
        }
    }
}

/// A [Read] object.
pub struct Readable<'a, W> {
    obj: &'a mut W,
    vtable: &'a ReadVTable<W>,
}
impl<'a, W> Read for Readable<'a, W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (self.vtable.read)(self.obj, buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        (self.vtable.read_exact)(self.obj, buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        (self.vtable.read_to_string)(self.obj, buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        (self.vtable.read_vectored)(self.obj, bufs)
    }
}

struct TryCloneVTable<O> {
    try_clone: fn(&O) -> io::Result<O>,
}
impl<O> Clone for TryCloneVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<O> Copy for TryCloneVTable<O> {}
impl<O: TryClone> TryCloneVTable<O> {
    pub const fn new() -> Self {
        Self {
            try_clone: O::try_clone,
        }
    }
}

// Allowing `clippy::type_complexity` on all of these as those are basically just
// signatures of the async functions that will only be used there and nowhere else.

/// An [AsyncSeek] object.
#[cfg(feature = "futures")]
pub struct AsyncSeekable<'a, W> {
    obj: &'a mut W,
    vtable: &'a AsyncSeekVTable<W>,
}
#[cfg(feature = "futures")]
#[allow(clippy::type_complexity)]
impl<'a, W> AsyncSeek for AsyncSeekable<'a, W> {
    fn poll_seek(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> std::task::Poll<io::Result<u64>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_seek;
        let obj = &mut this.obj;
        f(pin!(obj), cx, pos)
    }
}

#[cfg(feature = "futures")]
struct AsyncSeekVTable<O> {
    poll_seek: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> std::task::Poll<io::Result<u64>>,
}
#[cfg(feature = "futures")]
impl<O> Clone for AsyncSeekVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
#[cfg(feature = "futures")]
impl<O> Copy for AsyncSeekVTable<O> {}
#[cfg(feature = "futures")]
impl<O: AsyncSeek> AsyncSeekVTable<O> {
    pub const fn new() -> Self {
        Self {
            poll_seek: O::poll_seek,
        }
    }
}

/// An [AsyncWrite] object.
#[cfg(feature = "futures")]
pub struct AsyncWriteable<'a, W> {
    obj: &'a mut W,
    vtable: &'a AsyncWriteVTable<W>,
}
#[cfg(feature = "futures")]
impl<'a, W> AsyncWrite for AsyncWriteable<'a, W> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_write;
        let obj = &mut this.obj;
        f(pin!(obj), cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_flush;
        let obj = &mut this.obj;
        f(pin!(obj), cx)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_close;
        let obj = &mut this.obj;
        f(pin!(obj), cx)
    }

    fn poll_write_vectored(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_write_vectored;
        let obj = &mut this.obj;
        f(pin!(obj), cx, bufs)
    }
}

#[cfg(feature = "futures")]
#[allow(clippy::type_complexity)]
struct AsyncWriteVTable<O> {
    poll_write: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>>,
    poll_flush: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>>,
    poll_close: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>>,
    poll_write_vectored: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> std::task::Poll<io::Result<usize>>,
}
#[cfg(feature = "futures")]
impl<O> Clone for AsyncWriteVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
#[cfg(feature = "futures")]
impl<O> Copy for AsyncWriteVTable<O> {}
#[cfg(feature = "futures")]
impl<O: AsyncWrite> AsyncWriteVTable<O> {
    pub const fn new() -> Self {
        Self {
            poll_write: O::poll_write,
            poll_flush: O::poll_flush,
            poll_close: O::poll_close,
            poll_write_vectored: O::poll_write_vectored,
        }
    }
}

/// An [AsyncRead] object.
#[cfg(feature = "futures")]
pub struct AsyncReadable<'a, W> {
    obj: &'a mut W,
    vtable: &'a AsyncReadVTable<W>,
}
#[cfg(feature = "futures")]
impl<'a, W> AsyncRead for AsyncReadable<'a, W> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_read;
        let obj = &mut this.obj;
        f(pin!(obj), cx, buf)
    }

    fn poll_read_vectored(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = &mut self.as_mut();
        let f = this.vtable.poll_read_vectored;
        let obj = &mut this.obj;
        f(pin!(obj), cx, bufs)
    }
}

#[cfg(feature = "futures")]
#[allow(clippy::type_complexity)]
struct AsyncReadVTable<O> {
    poll_read: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>>,
    poll_read_vectored: fn(
        this: std::pin::Pin<&mut O>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
    ) -> std::task::Poll<io::Result<usize>>,
}
#[cfg(feature = "futures")]
impl<O> Clone for AsyncReadVTable<O> {
    fn clone(&self) -> Self {
        *self
    }
}
#[cfg(feature = "futures")]
impl<O> Copy for AsyncReadVTable<O> {}
#[cfg(feature = "futures")]
impl<O: AsyncRead> AsyncReadVTable<O> {
    pub const fn new() -> Self {
        Self {
            poll_read: O::poll_read,
            poll_read_vectored: O::poll_read_vectored,
        }
    }
}

/// A generic wrapper over IO operations.
pub struct GenericIo<W> {
    obj: W,
    write: Option<WriteVTable<W>>,
    read: Option<ReadVTable<W>>,
    seek: Option<SeekVTable<W>>,
    #[cfg(feature = "futures")]
    async_seek: Option<AsyncSeekVTable<W>>,
    #[cfg(feature = "futures")]
    async_write: Option<AsyncWriteVTable<W>>,
    #[cfg(feature = "futures")]
    async_read: Option<AsyncReadVTable<W>>,
    try_clone: Option<TryCloneVTable<W>>,
}
impl<W> GenericIo<W> {
    /// Create a new [GenericIo] that doesn't have support for anything.
    pub fn new_empty(obj: W) -> Self {
        Self {
            obj,
            write: None,
            read: None,
            seek: None,
            #[cfg(feature = "futures")]
            async_seek: None,
            #[cfg(feature = "futures")]
            async_write: None,
            #[cfg(feature = "futures")]
            async_read: None,
            try_clone: None,
        }
    }

    /// Obtain a [Write] implementation for this [GenericIo].
    ///
    /// In order to have access to seeking, [GenericIo::make_writeable]
    /// must have been called. Otherwise, [None] will be returned.
    pub const fn writeable(&mut self) -> Option<Writeable<'_, W>> {
        match &self.write {
            Some(vtable) => Some(Writeable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericIo] is writeable.
    pub const fn is_writeable(&self) -> bool {
        self.write.is_some()
    }

    /// Obtain a [Read] implementation for this [GenericIo].
    ///
    /// In order to have access to seeking, [GenericIo::make_readable]
    /// must have been called. Otherwise, [None] will be returned.
    pub const fn readable(&mut self) -> Option<Readable<'_, W>> {
        match &self.read {
            Some(vtable) => Some(Readable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericIo] is readable.
    pub const fn is_readable(&self) -> bool {
        self.read.is_some()
    }

    /// Obtain a [Seek] implementation for this [GenericIo].
    ///
    /// In order to have access to seeking, [GenericIo::make_seekable]
    /// must have been called. Otherwise, [None] will be returned.
    pub const fn seekable(&mut self) -> Option<Seekable<'_, W>> {
        match &self.seek {
            Some(vtable) => Some(Seekable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericIo] is seekable.
    pub const fn is_seekable(&self) -> bool {
        self.seek.is_some()
    }

    /// Obtain an [AsyncSeek] implementation for this [GenericWrite].
    ///
    /// In order to have access to seeking, [GenericWrite::make_async_seekable]
    /// must have been called. Otherwise, [None] will be returned.
    #[cfg(feature = "futures")]
    pub const fn async_seekable(&mut self) -> Option<AsyncSeekable<'_, W>> {
        match &self.async_seek {
            Some(vtable) => Some(AsyncSeekable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericWrite] is seekable.
    #[cfg(feature = "futures")]
    pub const fn is_async_seekable(&self) -> bool {
        self.async_seek.is_some()
    }

    /// Obtain an [AsyncWrite] implementation for this [GenericWrite].
    ///
    /// In order to have access to seeking, [GenericWrite::make_async_writeable]
    /// must have been called. Otherwise, [None] will be returned.
    #[cfg(feature = "futures")]
    pub const fn async_writeable(&mut self) -> Option<AsyncWriteable<'_, W>> {
        match &self.async_write {
            Some(vtable) => Some(AsyncWriteable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericWrite] is writeable.
    #[cfg(feature = "futures")]
    pub const fn is_async_writeable(&self) -> bool {
        self.async_write.is_some()
    }

    /// Obtain an [AsyncRead] implementation for this [GenericIo].
    ///
    /// In order to have access to reading, [GenericIo::make_async_readable]
    /// must have been called. Otherwise, [None] will be returned.
    #[cfg(feature = "futures")]
    pub const fn async_readable(&mut self) -> Option<AsyncReadable<'_, W>> {
        match &self.async_read {
            Some(vtable) => Some(AsyncReadable {
                obj: &mut self.obj,
                vtable,
            }),
            None => None,
        }
    }
    /// Check if this [GenericRead] is readable.
    #[cfg(feature = "futures")]
    pub const fn is_async_readable(&self) -> bool {
        self.async_read.is_some()
    }

    /// Try to clone this [GenericWrite].
    ///
    /// In order to have access to cloning, [GenericWrite::make_try_cloneable]
    /// must have been called. Otherwise, [None] will be returned.
    pub fn try_clone(&self) -> Option<io::Result<GenericIo<W>>> {
        self.try_clone.as_ref().map(|x| {
            (x.try_clone)(&self.obj).map(|obj| GenericIo {
                obj,
                write: self.write,
                read: self.read,
                seek: self.seek,
                #[cfg(feature = "futures")]
                async_seek: self.async_seek,
                #[cfg(feature = "futures")]
                async_write: self.async_write,
                #[cfg(feature = "futures")]
                async_read: self.async_read,
                try_clone: self.try_clone,
            })
        })
    }
    /// Check if this [GenericWrite] can be cloned.
    pub const fn can_try_clone(&self) -> bool {
        self.try_clone.is_some()
    }
}
impl<W: Read> GenericIo<W> {
    /// Make this [GenericIo] readable.
    ///
    /// This allows the use of [GenericIo::readable] and
    /// makes [GenericIo::is_readable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_readable(&mut self) {
        self.read = Some(ReadVTable::new());
    }

    /// Make this [GenericIo] readable.
    ///
    /// Unlike the [GenericIo::make_readable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::readable] and
    /// makes [GenericIo::is_readable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_readable(mut self) -> Self {
        self.make_readable();
        self
    }
}
impl<W: Write> GenericIo<W> {
    /// Make this [GenericIo] writeable.
    ///
    /// This allows the use of [GenericIo::writeable] and
    /// makes [GenericIo::is_writeable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_writeable(&mut self) {
        self.write = Some(WriteVTable::new());
    }

    /// Make this [GenericIo] writeable.
    ///
    /// Unlike the [GenericIo::make_writeable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::writeable] and
    /// makes [GenericIo::is_writeable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_writeable(mut self) -> Self {
        self.make_writeable();
        self
    }
}
impl<W: Seek> GenericIo<W> {
    /// Make this [GenericIo] seekable.
    ///
    /// This allows the use of [GenericIo::seekable] and
    /// makes [GenericIo::is_seekable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_seekable(&mut self) {
        self.seek = Some(SeekVTable::new());
    }

    /// Make this [GenericIo] seekable.
    ///
    /// Unlike the [GenericIo::make_seekable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::seekable] and
    /// makes [GenericIo::is_seekable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_seekable(mut self) -> Self {
        self.make_seekable();
        self
    }
}
#[cfg(feature = "futures")]
impl<W: AsyncSeek> GenericIo<W> {
    /// Make this [GenericIo] seekable.
    ///
    /// This allows the use of [GenericIo::async_seekable] and
    /// makes [GenericIo::is_async_seekable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_async_seekable(&mut self) {
        self.async_seek = Some(AsyncSeekVTable::new());
    }

    /// Make this [GenericIo] seekable.
    ///
    /// Unlike the [GenericIo::make_async_seekable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::async_seekable] and
    /// makes [GenericIo::is_async_seekable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_async_seekable(mut self) -> Self {
        self.make_async_seekable();
        self
    }
}
#[cfg(feature = "futures")]
impl<W: AsyncWrite> GenericIo<W> {
    /// Make this [GenericIo] writeable.
    ///
    /// This allows the use of [GenericIo::async_writeable] and
    /// makes [GenericIo::is_async_writeable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_async_writeable(&mut self) {
        self.async_write = Some(AsyncWriteVTable::new());
    }

    /// Make this [GenericIo] writeable.
    ///
    /// Unlike the [GenericIo::make_async_writeable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::async_writeable] and
    /// makes [GenericIo::is_async_writeable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_async_writeable(mut self) -> Self {
        self.make_async_writeable();
        self
    }
}
#[cfg(feature = "futures")]
impl<W: AsyncRead> GenericIo<W> {
    /// Make this [GenericIo] readable.
    ///
    /// This allows the use of [GenericIo::async_readable] and
    /// makes [GenericIo::is_async_readable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_async_readable(&mut self) {
        self.async_read = Some(AsyncReadVTable::new());
    }

    /// Make this [GenericIo] readable.
    ///
    /// Unlike the [GenericIo::make_async_readable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::async_readable] and
    /// makes [GenericIo::is_async_readable] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_async_readable(mut self) -> Self {
        self.make_async_readable();
        self
    }
}
impl<W: TryClone> GenericIo<W> {
    /// Make this [GenericIo] seekable.
    ///
    /// This allows the use of [GenericIo::try_clone] and
    /// makes [GenericIo::can_try_clone] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_try_cloneable(&mut self) {
        self.try_clone = Some(TryCloneVTable::new());
    }

    /// Make this [GenericIo] seekable.
    ///
    /// Unlike the [GenericIo::make_try_cloneable] method, this
    /// one accepts an owned object instead of a mutable reference.
    ///
    /// This allows the use of [GenericIo::try_clone] and
    /// makes [GenericIo::can_try_clone] return true.
    ///
    /// Since this function has a consistent output, calling
    /// it multiple times is effectively a noop.
    pub const fn make_into_try_cloneable(mut self) -> Self {
        self.make_try_cloneable();
        self
    }
}

macro_rules! write_ext {
    ($(fn $name:ident(&mut self, $ty:ty);)* $(;)?) => {
        pub trait WriteExt {$(
            fn $name(&mut self, value: $ty) -> io::Result<()>;
        )*}
        impl<W: Write> WriteExt for W {$(
            fn $name(&mut self, value: $ty) -> io::Result<()> {
                self.write_all(&value.to_le_bytes())
            }
        )*}
    };
}
write_ext! {
    fn write_i8(&mut self, i8);
    fn write_i16_le(&mut self, i16);
    fn write_i32_le(&mut self, i32);
    fn write_i64_le(&mut self, i64);
    fn write_i128_le(&mut self, i128);
    fn write_u8(&mut self, u8);
    fn write_u16_le(&mut self, u16);
    fn write_u32_le(&mut self, u32);
    fn write_u64_le(&mut self, u64);
    fn write_u128_le(&mut self, u128);
    fn write_f32_le(&mut self, f32);
    fn write_f64_le(&mut self, f64);
}

macro_rules! read_ext {
    ($(fn $name:ident(&mut self, $ty:ty);)* $(;)?) => {
        pub trait ReadExt {$(
            fn $name(&mut self) -> io::Result<$ty>;
        )*}
        impl<R: Read> ReadExt for R {$(
            fn $name(&mut self) -> io::Result<$ty> {
                let mut buf = [0; size_of::<$ty>()];
                self.read_exact(&mut buf)?;
                Ok(<$ty>::from_le_bytes(buf))
            }
        )*}
    };
}
read_ext! {
    fn read_i8(&mut self, i8);
    fn read_i16_le(&mut self, i16);
    fn read_i32_le(&mut self, i32);
    fn read_i64_le(&mut self, i64);
    fn read_i128_le(&mut self, i128);
    fn read_u8(&mut self, u8);
    fn read_u16_le(&mut self, u16);
    fn read_u32_le(&mut self, u32);
    fn read_u64_le(&mut self, u64);
    fn read_u128_le(&mut self, u128);
    fn read_f32_le(&mut self, f32);
    fn read_f64_le(&mut self, f64);
}

macro_rules! awrite_ext {
    ($(fn $name:ident(&mut self, $ty:ty);)* $(;)?) => {
        pub trait AsyncWriteExt {$(
            fn $name(&mut self, value: $ty) -> impl Future<Output = io::Result<()>>;
        )*}
        impl<W: futures_io::AsyncWrite + std::marker::Unpin> AsyncWriteExt for W {$(
            async fn $name(&mut self, value: $ty) -> io::Result<()> {
                use futures_util::AsyncWriteExt as _;
                self.write_all(&value.to_le_bytes()).await
            }
        )*}
    };
}
#[cfg(feature = "futures")]
awrite_ext! {
    fn write_i8(&mut self, i8);
    fn write_i16_le(&mut self, i16);
    fn write_i32_le(&mut self, i32);
    fn write_i64_le(&mut self, i64);
    fn write_i128_le(&mut self, i128);
    fn write_u8(&mut self, u8);
    fn write_u16_le(&mut self, u16);
    fn write_u32_le(&mut self, u32);
    fn write_u64_le(&mut self, u64);
    fn write_u128_le(&mut self, u128);
    fn write_f32_le(&mut self, f32);
    fn write_f64_le(&mut self, f64);
}

macro_rules! aread_ext {
    ($(fn $name:ident(&mut self, $ty:ty);)* $(;)?) => {
        pub trait AsyncReadExt {$(
            fn $name(&mut self) -> impl Future<Output = io::Result<$ty>>;
        )*}
        impl<R: futures_io::AsyncRead + std::marker::Unpin> AsyncReadExt for R {$(
            async fn $name(&mut self) -> io::Result<$ty> {
                use futures_util::AsyncReadExt as _;
                let mut buf = [0; size_of::<$ty>()];
                self.read_exact(&mut buf).await?;
                Ok(<$ty>::from_le_bytes(buf))
            }
        )*}
    };
}
#[cfg(feature = "futures")]
aread_ext! {
    fn read_i8(&mut self, i8);
    fn read_i16_le(&mut self, i16);
    fn read_i32_le(&mut self, i32);
    fn read_i64_le(&mut self, i64);
    fn read_i128_le(&mut self, i128);
    fn read_u8(&mut self, u8);
    fn read_u16_le(&mut self, u16);
    fn read_u32_le(&mut self, u32);
    fn read_u64_le(&mut self, u64);
    fn read_u128_le(&mut self, u128);
    fn read_f32_le(&mut self, f32);
    fn read_f64_le(&mut self, f64);
}
