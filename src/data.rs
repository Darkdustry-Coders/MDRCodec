// use std::os::raw::c_void;

#[derive(PartialEq, Eq)]
pub enum ChunkKind {
    /// A jump table.
    Jmp,
}
impl ChunkKind {
    /// Get a numeric name of the chunk.
    #[must_use]
    #[inline(always)]
    pub const fn ordinal(&self) -> u8 {
        match self {
            Self::Jmp => 1,
        }
    }

    /// Obtain a chunk kind from its numeric name.
    ///
    /// Returns the input on failure.
    #[inline(always)]
    pub const fn from_ordinal(value: u8) -> Result<ChunkKind, u8> {
        match value {
            1 => Ok(ChunkKind::Jmp),
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

#[repr(C)]
pub struct Building {
    pub item_types: *const [u16],
    pub item_counts: *const [u32],
    pub item_count: u16,
}

#[repr(C)]
pub struct Tile {
    pub building: *const Building,
    pub data_extra: u32,
    pub block: u16,
    pub floor: u16,
    pub overlay: u16,
    pub data_block: u8,
    pub data_floor: u8,
    pub data_overlay: u8,
}

#[repr(C)]
pub struct World {
    tiles: *const Tile,
    width: u32,
    height: u32,
}
impl World {
    pub const fn tile(&self, width: i32, height: i32) -> Option<&Tile> {
        if width < 0 || height < 0 || width as u32 > self.width || height as u32 > self.height {
            return None;
        }

        unsafe {
            let idx = width as usize + (height as usize * self.width as usize);
            return Some(self.tiles.add(idx).as_ref().unwrap());
        }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }
}
