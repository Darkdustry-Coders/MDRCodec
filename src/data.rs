// use std::os::raw::c_void;

#[derive(PartialEq, Eq)]
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
