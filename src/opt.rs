use std::io::{self, Read, Write};

use crate::io::{ReadExt, WriteExt};

pub struct CountingWrite<W> {
    write: W,
    counter: usize,
}
impl<W: Write> CountingWrite<W> {
    pub const fn new(write: W) -> Self {
        Self { write, counter: 0 }
    }

    /// Finish counting.
    pub fn finish(self) -> (W, usize) {
        (self.write, self.counter)
    }
}
impl<W: Write> Write for CountingWrite<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.write.write(buf)?;
        self.counter += len;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.write.flush()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Compression {
    /// No compression.
    None,

    /// Use lz4 algorithm.
    ///
    /// `lz4` feature must be enabled.
    #[cfg(feature = "lz4")]
    Lz4 { mode: lz4::block::CompressionMode },

    /// Use deflate algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[cfg(feature = "flate2")]
    Deflate { quality: flate2::Compression },

    /// Use zlib algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[cfg(feature = "flate2")]
    Zlib { quality: flate2::Compression },

    /// Use gzip algorithm.
    ///
    /// `flate2` feature must be enabled.
    #[cfg(feature = "flate2")]
    Gzip { quality: flate2::Compression },
}
impl Compression {
    pub const fn ordinal(&self) -> u8 {
        match self {
            Self::None => 0,
            #[cfg(feature = "lz4")]
            Self::Lz4 { .. } => 1,
            #[cfg(feature = "flate2")]
            Self::Deflate { .. } => 2,
            #[cfg(feature = "flate2")]
            Self::Zlib { .. } => 3,
            #[cfg(feature = "flate2")]
            Self::Gzip { .. } => 4,
        }
    }

    /// Write arbitrary data.
    ///
    /// Caller must ensure that the data is readable.
    pub fn write_data<W: Write>(&self, mut write: W, buf: &[u8]) -> io::Result<usize> {
        match self {
            Compression::None => {
                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                write.write_all(buf)?;

                Ok(buf.len())
            }
            #[cfg(feature = "lz4")]
            Compression::Lz4 { mode } => {
                let buf = lz4::block::compress(buf, Some(*mode), true)?;
                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                write.write_all(&buf)?;

                Ok(buf.len())
            }
            #[cfg(feature = "flate2")]
            Compression::Deflate { quality } => {
                use std::io::Cursor;

                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                let mut vec = vec![];
                let mut w = flate2::write::DeflateEncoder::new(Cursor::new(&mut vec), *quality);
                w.write_all(buf)?;
                w.finish()?;

                if vec.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", vec.len(), u32::MAX),
                    ));
                }

                write.write_all(&vec)?;

                Ok(vec.len())
            }
            #[cfg(feature = "flate2")]
            Compression::Zlib { quality } => {
                use std::io::Cursor;

                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                let mut vec = vec![];
                let mut w = flate2::write::ZlibEncoder::new(Cursor::new(&mut vec), *quality);
                w.write_all(buf)?;
                w.finish()?;

                if vec.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", vec.len(), u32::MAX),
                    ));
                }

                write.write_all(&vec)?;

                Ok(vec.len())
            }
            #[cfg(feature = "flate2")]
            Compression::Gzip { quality } => {
                use std::io::Cursor;

                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                let mut vec = vec![];
                let mut w = flate2::write::GzEncoder::new(Cursor::new(&mut vec), *quality);
                w.write_all(buf)?;
                w.finish()?;

                if vec.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", vec.len(), u32::MAX),
                    ));
                }

                write.write_all(&vec)?;

                Ok(vec.len())
            }
        }
    }

    pub fn read_data<R: Read>(&self, mut read: R, len: u32) -> io::Result<Vec<u8>> {
        match self {
            Compression::None => {
                let mut data = Vec::with_capacity(len as usize);
                read.read_to_end(&mut data)?;
                data.shrink_to_fit();
                Ok(data)
            }
            #[cfg(feature = "lz4")]
            Compression::Lz4 { .. } => {
                let mut buf = Vec::with_capacity(len as usize);
                read.read_to_end(&mut buf)?;
                let buf = lz4::block::decompress(&buf, None)?;
                Ok(buf)
            }
            #[cfg(feature = "flate2")]
            Compression::Deflate { .. } => {
                let len = read.read_u32_le()? as u64;
                let mut read = flate2::read::DeflateDecoder::new(read.take(len));
                let mut buf = vec![];
                read.read_to_end(&mut buf)?;
                Ok(buf)
            }
            #[cfg(feature = "flate2")]
            Compression::Zlib { .. } => {
                let len = read.read_u32_le()? as u64;
                let mut read = flate2::read::ZlibDecoder::new(read.take(len));
                let mut buf = vec![];
                read.read_to_end(&mut buf)?;
                Ok(buf)
            }
            #[cfg(feature = "flate2")]
            Compression::Gzip { .. } => {
                let len = read.read_u32_le()? as u64;
                let mut read = flate2::read::GzDecoder::new(read.take(len));
                let mut buf = vec![];
                read.read_to_end(&mut buf)?;
                Ok(buf)
            }
        }
    }

    pub fn write<W: Write>(&self, mut to: W) -> io::Result<()> {
        use crate::io::WriteExt as _;

        to.write_u8(self.ordinal())?;
        match self {
            Self::None => (),
            #[cfg(feature = "lz4")]
            Self::Lz4 { mode } => match mode {
                lz4::block::CompressionMode::HIGHCOMPRESSION(x) => {
                    to.write_u8(2)?;
                    to.write_i32_le(*x)?;
                }
                lz4::block::CompressionMode::FAST(x) => {
                    to.write_u8(1)?;
                    to.write_i32_le(*x)?;
                }
                lz4::block::CompressionMode::DEFAULT => {
                    to.write_u8(0)?;
                }
            },
            #[cfg(feature = "flate2")]
            Self::Deflate { quality } | Self::Zlib { quality } | Self::Gzip { quality } => {
                to.write_u32_le(quality.level())?;
            }
        }
        Ok(())
    }

    pub fn read<R: Read>(mut from: R) -> io::Result<Self> {
        use crate::io::ReadExt as _;

        match from.read_u8()? {
            0 => Ok(Self::None),

            #[cfg(not(feature = "lz4"))]
            1 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'lz4' must be enabled to handle lz4 compression",
            )),
            #[cfg(feature = "lz4")]
            1 => {
                Ok(Self::Lz4 {
                    mode: match from.read_u8()? {
                        2 => lz4::block::CompressionMode::HIGHCOMPRESSION(from.read_i32_le()?),
                        1 => lz4::block::CompressionMode::FAST(from.read_i32_le()?),
                        0 => lz4::block::CompressionMode::DEFAULT,
                        x => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!("Unsupported lz4 compression mode: {x}"),
                            ));
                        }
                    },
                })
            }

            #[cfg(not(feature = "flate2"))]
            2 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle deflate compression",
            )),
            #[cfg(feature = "flate2")]
            2 => Ok(Self::Deflate {
                quality: flate2::Compression::new(from.read_u32_le()?),
            }),

            #[cfg(not(feature = "flate2"))]
            3 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle zlib compression",
            )),
            #[cfg(feature = "flate2")]
            3 => Ok(Self::Zlib {
                quality: flate2::Compression::new(from.read_u32_le()?),
            }),

            #[cfg(not(feature = "flate2"))]
            4 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle gzip compression",
            )),
            #[cfg(feature = "flate2")]
            4 => Ok(Self::Gzip {
                quality: flate2::Compression::new(from.read_u32_le()?),
            }),

            x => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported compression format: {x}"),
            )),
        }
    }

    #[cfg(feature = "futures")]
    pub async fn write_async<W: futures_io::AsyncWrite + std::marker::Unpin>(&self, mut to: W) -> io::Result<()> {
        use crate::io::AsyncWriteExt as _;

        to.write_u8(self.ordinal()).await?;
        match self {
            Self::None => (),
            #[cfg(feature = "lz4")]
            Self::Lz4 { mode } => match mode {
                lz4::block::CompressionMode::HIGHCOMPRESSION(x) => {
                    to.write_u8(2).await?;
                    to.write_i32_le(*x).await?;
                }
                lz4::block::CompressionMode::FAST(x) => {
                    to.write_u8(1).await?;
                    to.write_i32_le(*x).await?;
                }
                lz4::block::CompressionMode::DEFAULT => {
                    to.write_u8(0).await?;
                }
            },
            #[cfg(feature = "flate2")]
            Self::Deflate { quality } | Self::Zlib { quality } | Self::Gzip { quality } => {
                to.write_u32_le(quality.level()).await?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "futures")]
    pub async fn read_async<R: futures_io::AsyncRead + std::marker::Unpin>(mut from: R) -> io::Result<Self> {
        use crate::io::AsyncReadExt as _;

        match from.read_u8().await? {
            0 => Ok(Self::None),

            #[cfg(not(feature = "lz4"))]
            1 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'lz4' must be enabled to handle lz4 compression",
            )),
            #[cfg(feature = "lz4")]
            1 => {
                Ok(Self::Lz4 {
                    mode: match from.read_u8().await? {
                        2 => lz4::block::CompressionMode::HIGHCOMPRESSION(from.read_i32_le().await?),
                        1 => lz4::block::CompressionMode::FAST(from.read_i32_le().await?),
                        0 => lz4::block::CompressionMode::DEFAULT,
                        x => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!("Unsupported lz4 compression mode: {x}"),
                            ));
                        }
                    },
                })
            }

            #[cfg(not(feature = "flate2"))]
            2 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle deflate compression",
            )),
            #[cfg(feature = "flate2")]
            2 => Ok(Self::Deflate {
                quality: flate2::Compression::new(from.read_u32_le().await?),
            }),

            #[cfg(not(feature = "flate2"))]
            3 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle zlib compression",
            )),
            #[cfg(feature = "flate2")]
            3 => Ok(Self::Zlib {
                quality: flate2::Compression::new(from.read_u32_le().await?),
            }),

            #[cfg(not(feature = "flate2"))]
            4 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle gzip compression",
            )),
            #[cfg(feature = "flate2")]
            4 => Ok(Self::Gzip {
                quality: flate2::Compression::new(from.read_u32_le().await?),
            }),

            x => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported compression format: {x}"),
            )),
        }
    }
}
