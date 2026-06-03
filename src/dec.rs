//! MDR decoder.

use std::{io::{self, Cursor, Read, Seek, SeekFrom}, time::Duration};

use crate::{data::{Chunk, ChunkBody, ChunkKind, IdChunkv1, MapChunkv1, Tilev1}, io::{GenericIo, ReadExt, TryClone}, opt::Compression};

#[allow(unused)]
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
#[allow(unused)]
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

#[allow(unused)]
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
#[allow(unused)]
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

const PAST_HEADER: usize = 8 + 8 + 4 + 1;

pub struct Builder<R> {
    read: GenericIo<R>,
    validate: bool,
}
impl<R> Builder<R> {
    #[must_use]
    pub fn new(read: R) -> Self {
        Self {
            read: GenericIo::new_empty(read),
            validate: true,
        }
    }

    /// Skip chunk validation.
    ///
    /// ## Safety
    ///
    /// Since this leads to execution of unsafe code on unchecked data,
    /// this method is marked as unsafe.
    pub unsafe fn skip_validation(mut self) -> Self {
        self.validate = false;
        self
    }

    pub fn build(mut self) -> io::Result<Decoder<R>> {
        if !self.read.is_readable() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "backend does not support 'Read'",
            ))
        }

        #[cfg(feature = "futures")]
        if self.read.is_readable() && self.read.is_async_readable() && (self.read.is_seekable() != self.read.is_async_seekable()) {
            if self.read.is_seekable() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'AsyncSeek'",
                ))
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "backend does not support 'Seek'",
                ))
            }
        }

        let mut buf = [0; 4];
        sread!(self).read_exact(&mut buf)?;
        if &buf != b"MDR\0" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing file magic",
            ));
        }
        let version = sread!(self).read_u16_le()?;
        if version != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("version {version} is not supported")));
        }
        let compression = Compression::read(sread!(self))?;
        let mut buf = [0; PAST_HEADER];
        sread!(self).read_exact(&mut buf)?;
        if let Some(i) = buf.iter().enumerate().find(|(_, x)| **x != 0).map(|(i, _)| i) {
            for x in buf.iter() {
                let a = *x / 16;
                let b = *x % 16;
                print!("{}{} ", if a < 10 { b'0' + a } else { b'a' + a - 10 } as char, if b < 10 { b'0' + b } else { b'a' + b - 10 } as char);
            }
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("a non-zero byte in stop chunk header at {i}")));
        }

        let enc = Decoder {
            read: self.read,
            validate: self.validate,
            compression,
        };

        Ok(enc)
    }
}
impl<R: TryClone> Builder<R> {
    pub const fn try_cloneable(mut self) -> Self {
        self.read.make_try_cloneable();
        self
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

/// Filter options.
pub trait Filter {
    fn chunk_kind(&self) -> Option<ChunkKind>;
}
impl Filter for ChunkKind {
    fn chunk_kind(&self) -> Option<ChunkKind> {
        Some(*self)
    }
}

/// MDR decoder.
///
/// Importantly, the [Iterator] (and its async counterpart) implementations are
/// very expensive and shouldn't be used for seeking.
pub struct Decoder<R> {
    /// Writing destination.
    read: GenericIo<R>,
    /// Whether encoder should validate the input.
    validate: bool,
    /// Compression used in this file.
    compression: Compression,
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
impl<R> Iterator for Decoder<R> {
    type Item = io::Result<Chunk>;

    fn next(&mut self) -> Option<Self::Item> {
        let seekable = self.read.is_seekable();

        macro_rules! catch {
            ($expr:expr) => {
                match $expr {
                    Ok(x) => x,
                    Err(why) => return Some(Err(why)),
                }
            };
        }

        loop {
            let mut read = match self.read.readable() {
                Some(x) => x,
                None => {
                    return Some(Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "backend does not support 'Read'",
                    )))
                }
            };
            let kind = {
                let mut buf = [0];
                match read.read(&mut buf) {
                    Ok(0) => return None,
                    Ok(_) => buf[0],
                    Err(why) => return Some(Err(why)),
                }
            };
            let timestamp = Duration::from_millis(catch!(read.read_u64_le()));
            let len = catch!(read.read_u32_le());
            let mut buf = vec![0; len as usize];
            if let Err(why) = read.read_exact(&mut buf) { return Some(Err(why)) };
            
            if seekable {
                if let Err(why) = match self.read.seekable() {
                    Some(x) => x,
                    None => {
                        return Some(Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "backend does not support 'Seek'",
                        )))
                    }
                }.seek(SeekFrom::Current(PAST_HEADER as i64)) { return Some(Err(why)) }
            } else {
                if let Err(why) = read.read_exact(&mut [0; PAST_HEADER]) { return Some(Err(why)) }
            };

            match kind {
                1 => {
                    // TODO: jmp chunk, deal with it later.
                }
                2 => {
                    let buf = match self.compression {
                        Compression::None => buf,
                        x => { let len = buf.len(); match x.read_data(Cursor::new(buf), len as u32) {
                            Ok(x) => x,
                            Err(why) => return Some(Err(why)),
                        } },
                    };
                    let mut read = Cursor::new(buf);
                    let width = catch!(read.read_u32_le());
                    let height = catch!(read.read_u32_le());
                    let mut tiles = Vec::with_capacity(width as usize * height as usize);
                    for _ in 0..width as usize * height as usize {
                        let mut tile = Tilev1::zeroed();
                        catch!(tile.read(&mut read));
                        tiles.push(tile);
                    }
                    return Some(Ok(Chunk { timestamp, body: ChunkBody::Mapv1(MapChunkv1 {
                        width: width,
                        body: tiles.into_boxed_slice(),
                    }) }))
                },
                3 => {
                    let buf = match self.compression {
                        Compression::None => buf,
                        x => { let len = buf.len(); match x.read_data(Cursor::new(buf), len as u32) {
                            Ok(x) => x,
                            Err(why) => return Some(Err(why)),
                        } },
                    };
                    if self.validate {
                        let mut cursor = Cursor::new(&buf);
                        catch!(cursor.read_u8());
                        loop {
                            {
                                let mut b = [0, 0];
                                match cursor.read(&mut b) {
                                    Ok(0) => break,
                                    Ok(1) => return Some(Err(io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill the whole buffer"))),
                                    Ok(_) => (),
                                    Err(why) => return Some(Err(why)),
                                }
                            };
                            let len = catch!(cursor.read_u8());
                            if len == 0 {
                                return Some(Err(io::Error::new(io::ErrorKind::InvalidData, "content name cannot be empty")));
                            }
                            for _ in 0..len {
                                let x = catch!(cursor.read_u8());
                                if !x.is_ascii_alphanumeric() && x != b'_' && x != b'-' {
                                    return Some(Err(io::Error::new(io::ErrorKind::InvalidData, "invalid content name")));
                                }
                            }
                        }
                    }
                    return Some(Ok(Chunk { timestamp, body: ChunkBody::Idv1(IdChunkv1 {
                        body: buf,
                    }) }))
                }
                4 => todo!(),
                x => {
                    return Some(Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid chunk {x}"))));
                }
            }
        }
    }
}
