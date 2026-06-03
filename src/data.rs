// use std::os::raw::c_void;

use std::{fmt, io::{self, Read}, ops, time::Duration};

use crate::io::ReadExt;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ChunkKind {
    /// A jump table.
    Jmp,
    /// A map snapshot table.
    Map,
    /// An ID table.
    Id,
    /// A modifications table chunk.
    Mod,
}
impl fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ChunkKind::Jmp => "JMP",
            ChunkKind::Map => "MAP",
            ChunkKind::Id => "ID",
            ChunkKind::Mod => "MOD",
        })
    }
}
impl ChunkKind {
    /// Get a numeric name of the chunk.
    #[must_use]
    #[inline(always)]
    pub const fn ordinal(&self) -> u8 {
        match self {
            Self::Jmp => 1,
            Self::Map => 2,
            Self::Id => 3,
            Self::Mod => 4,
        }
    }

    /// Obtain a chunk kind from its numeric name.
    ///
    /// Returns the input on failure.
    #[inline(always)]
    pub const fn from_ordinal(value: u8) -> Result<ChunkKind, u8> {
        match value {
            1 => Ok(ChunkKind::Jmp),
            2 => Ok(ChunkKind::Map),
            3 => Ok(ChunkKind::Id),
            4 => Ok(ChunkKind::Mod),
            x => Err(x),
        }
    }
}

pub struct IdChunkv1 {
    pub(crate) body: Vec<u8>,
}
impl IdChunkv1 {
    pub const fn content_type(&self) -> u8 { self.body.as_slice()[0] }
    pub fn entries(&self) -> IdChunkv1Iter<'_> {
        IdChunkv1Iter { body: &self.body[1..] }
    }
}
pub struct IdChunkv1Iter<'a> {
    body: &'a [u8],
}
impl<'a> Iterator for IdChunkv1Iter<'a> {
    type Item = (u16, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.body.read_u16_le().ok()?;
        let len = self.body.read_u8().ok()?;
        let str = unsafe { str::from_utf8_unchecked(&self.body[..len as usize]) };
        self.body = &self.body[len as usize..];
        Some((id, str))
    }
}

pub struct Tilev1 {
    pub block_id: u16,
    pub floor_id: u16,
    pub overlay_id: u16,
    pub block_data: u8,
    pub floor_data: u8,
    pub overlay_data: u8,
    pub extra_data: u32,
}
impl Tilev1 {
    pub const fn zeroed() -> Self {
        // Everything can be 0 I don't care.
        unsafe { std::mem::zeroed() }
    }

    pub fn read<R: Read>(&mut self, mut read: R) -> io::Result<()> {
        self.block_id = read.read_u16_le()?;
        self.floor_id = read.read_u16_le()?;
        self.overlay_id = read.read_u16_le()?;
        self.block_data = read.read_u8()?;
        self.floor_data = read.read_u8()?;
        self.overlay_data = read.read_u8()?;
        self.extra_data = read.read_u32_le()?;
        if read.read_u8()? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "buildings are not supported yet"));
        }
        Ok(())
    }
}

pub struct MapChunkv1 {
    pub(crate) width: u32,
    pub(crate) body: Box<[Tilev1]>,
}
impl MapChunkv1 {
    pub const fn width(&self) -> u32 { self.width }
    pub const fn height(&self) -> u32 { (self.body.len() / self.width as usize) as u32 }
    pub fn tiles(&self) -> &[Tilev1] { &self.body }
}
impl ops::Index<(u32, u32)> for MapChunkv1 {
    type Output = Tilev1;

    fn index(&self, index: (u32, u32)) -> &Self::Output {
        if index.0 >= self.width() || index.1 >= self.height() { panic!("index {index:?} is out of bounds {:?}", (self.width(), self.height())); }
        &self.tiles()[index.0 as usize + index.1 as usize * self.width as usize]
    }
}

pub struct Chunk {
    pub timestamp: Duration,
    pub body: ChunkBody,
}
impl Chunk {
    #[inline(always)]
    pub const fn kind(&self) -> ChunkKind {
        self.body.kind()
    }
}
pub enum ChunkBody {
    Mapv1(MapChunkv1),
    Idv1(IdChunkv1),
}
impl ChunkBody {
    pub const fn kind(&self) -> ChunkKind {
        match self {
            ChunkBody::Mapv1(_) => ChunkKind::Map,
            ChunkBody::Idv1(_) => ChunkKind::Id,
        }
    }
}

// unsafe extern "C" {
//     /// Allocate the amount of bytes specified in *length*.
//     ///
//     /// Returned pointer is guaranteed to have maximum align.
//     ///
//     /// Returns `null` if there's not enough RAM available.
//     unsafe fn malloc(length: usize) -> *mut c_void;
//     /// Deallocate the memory at pointer.
//     unsafe fn free(ptr: *mut c_void);
//     /// Reallocate the memory at pointer with new length.
//     ///
//     /// Kernel will attempt to simply grow the region instead of
//     /// fully reallocating it. If it can't, the contents will be
//     /// copied as it it was.
//     ///
//     /// On success, pointer to the new region is returned and the
//     /// old one is invalidated. Even if values are the same, use of
//     /// the old pointer is undefined behavior.
//     ///
//     /// On failure, `null` is returned and the old pointer is still
//     /// valid.
//     unsafe fn realloc(ptr: *mut c_void, new_length: usize) -> *mut c_void;
// }

pub struct ItemStack {
    pub id: u16,
    pub count: i32,
}

pub trait BuildingAccess {
    fn item_count(&self) -> usize;
    fn item(&self, idx: usize) -> Option<ItemStack>;
}

pub trait TileAccess {
    type BA<'a>: BuildingAccess + 'a where Self: 'a;
    fn building(&self) -> Option<Self::BA<'_>>;

    fn block(&self) -> u16;
    fn floor(&self) -> u16;
    fn overlay(&self) -> u16;
    fn data_block(&self) -> u8;
    fn data_floor(&self) -> u8;
    fn data_overlay(&self) -> u8;
    fn data_extra(&self) -> u32;
}

pub trait WorldAccess {
    fn width(&self) -> u32;
    fn height(&self) -> u32;

    type BA<'a>: BuildingAccess + 'a where Self: 'a;
    type TA<'a>: TileAccess<BA<'a> = Self::BA<'a>> + 'a where Self: 'a;
    fn tile(&self, x: u32, y: u32) -> Option<Self::TA<'_>>;
}
