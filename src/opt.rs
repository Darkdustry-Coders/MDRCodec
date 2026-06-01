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

#[derive(Debug)]
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
    pub fn write_data<W: Write>(&self, mut write: W, buf: &[u8]) -> io::Result<usize> {
        match self {
            Compression::None => {
                if buf.len() > u32::MAX as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("buffer too long ({} > {})", buf.len(), u32::MAX),
                    ));
                }

                write.write_u32_le(buf.len() as u32)?;
                write.write_all(buf)?;

                Ok(buf.len() + 4)
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

                write.write_u32_le(buf.len() as u32)?;
                write.write_all(&buf)?;

                Ok(buf.len() + 4)
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

                write.write_u32_le(vec.len() as u32)?;
                write.write_all(&vec)?;

                Ok(vec.len() + 4)
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

                write.write_u32_le(vec.len() as u32)?;
                write.write_all(&vec)?;

                Ok(vec.len() + 4)
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

                write.write_u32_le(vec.len() as u32)?;
                write.write_all(&vec)?;

                Ok(vec.len() + 4)
            }
        }
    }

    pub fn read_data<R: Read>(&self, mut read: R) -> io::Result<Vec<u8>> {
        match self {
            Compression::None => {
                let len = read.read_u32_le()? as usize;
                let mut buf = vec![0; len];
                read.read_exact(&mut buf)?;
                Ok(buf)
            }
            #[cfg(feature = "lz4")]
            Compression::Lz4 { .. } => {
                let len = read.read_u32_le()? as usize;
                let mut buf = vec![0; len];
                read.read_exact(&mut buf)?;
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
        to.write_all(&[self.ordinal()])?;
        match self {
            Self::None => (),
            #[cfg(feature = "lz4")]
            Self::Lz4 { mode } => match mode {
                lz4::block::CompressionMode::HIGHCOMPRESSION(x) => {
                    let mut buf = [2; 5];
                    buf[1..][..4].copy_from_slice(&x.to_le_bytes());
                    to.write_all(&buf)?;
                }
                lz4::block::CompressionMode::FAST(x) => {
                    let mut buf = [1; 5];
                    buf[1..][..4].copy_from_slice(&x.to_le_bytes());
                    to.write_all(&buf)?;
                }
                lz4::block::CompressionMode::DEFAULT => {
                    to.write_all(&[0])?;
                }
            },
            #[cfg(feature = "flate2")]
            Self::Deflate { quality } | Self::Zlib { quality } | Self::Gzip { quality } => {
                to.write_all(&quality.level().to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn read<R: Read>(mut from: R) -> io::Result<Self> {
        let mut version = [0];
        from.read_exact(&mut version)?;

        match version[0] {
            0 => Ok(Self::None),

            #[cfg(not(feature = "lz4"))]
            1 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'lz4' must be enabled to handle lz4 compression",
            )),
            #[cfg(feature = "lz4")]
            1 => {
                let mut buf = [0; 5];
                from.read_exact(&mut buf)?;
                let mut num = [0; 4];
                num.copy_from_slice(&buf[1..]);
                let num = i32::from_le_bytes(num);
                Ok(Self::Lz4 {
                    mode: match buf[0] {
                        2 => lz4::block::CompressionMode::HIGHCOMPRESSION(num),
                        1 => lz4::block::CompressionMode::FAST(num),
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
                quality: {
                    let mut buf = [0; 4];
                    from.read_exact(&mut buf)?;
                    flate2::Compression::new(u32::from_le_bytes(buf))
                },
            }),

            #[cfg(not(feature = "flate2"))]
            3 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle zlib compression",
            )),
            #[cfg(feature = "flate2")]
            3 => Ok(Self::Zlib {
                quality: {
                    let mut buf = [0; 4];
                    from.read_exact(&mut buf)?;
                    flate2::Compression::new(u32::from_le_bytes(buf))
                },
            }),

            #[cfg(not(feature = "flate2"))]
            4 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Feature 'flate2' must be enabled to handle gzip compression",
            )),
            #[cfg(feature = "flate2")]
            4 => Ok(Self::Gzip {
                quality: {
                    let mut buf = [0; 4];
                    from.read_exact(&mut buf)?;
                    flate2::Compression::new(u32::from_le_bytes(buf))
                },
            }),

            x => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported compression format: {x}"),
            )),
        }
    }
}
