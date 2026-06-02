use crate::data::{BuildingAccess, ItemStack, TileAccess, WorldAccess};

#[repr(C)]
pub struct FfiBuilding {
    pub item_types: *const u16,
    pub item_counts: *const i32,
    pub item_count: u16,
}
impl<'a> BuildingAccess for &'a FfiBuilding {
    fn item_count(&self) -> usize {
        self.item_count as usize
    }

    fn item(&self, idx: usize) -> Option<ItemStack> {
        let Ok(idx): Result<u16, _> = idx.try_into() else { return None; };
        if idx >= self.item_count { return None; }
        unsafe {
            Some(ItemStack {
                id: *self.item_types.add(idx as usize),
                count: *self.item_counts.add(idx as usize),
            })
        }
    }
}

#[repr(C)]
pub struct FfiTile {
    pub building: *const FfiBuilding,
    pub data_extra: u32,
    pub block: u16,
    pub floor: u16,
    pub overlay: u16,
    pub data_block: u8,
    pub data_floor: u8,
    pub data_overlay: u8,
}
impl<'a> TileAccess for &'a FfiTile {
    type BA<'b> = &'b FfiBuilding where Self: 'b;

    fn block(&self) -> u16 {
        self.block
    }

    fn floor(&self) -> u16 {
        self.floor
    }

    fn overlay(&self) -> u16 {
        self.overlay
    }

    fn data_block(&self) -> u8 {
        self.data_block
    }

    fn data_floor(&self) -> u8 {
        self.data_floor
    }

    fn data_overlay(&self) -> u8 {
        self.data_overlay
    }

    fn data_extra(&self) -> u32 {
        self.data_extra
    }

    fn building(&self) -> Option<Self::BA<'_>> {
        if self.building.is_null() { return None; }
        unsafe { Some(&*self.building) }
    }
}

#[repr(C)]
pub struct FfiWorld {
    tiles: *const FfiTile,
    width: u32,
    height: u32,
}
impl WorldAccess for FfiWorld {
    type BA<'a> = &'a FfiBuilding where Self: 'a;
    type TA<'a> = &'a FfiTile where Self: 'a;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn tile(&self, x: u32, y: u32) -> Option<Self::TA<'_>> {
        if x >= self.width || y >= self.height {
            return None;
        }

        unsafe {
            let idx = x as usize + (y as usize * self.width as usize);
            Some(self.tiles.add(idx).as_ref().unwrap())
        }
    }
}
impl WorldAccess for &FfiWorld {
    type BA<'a> = &'a FfiBuilding where Self: 'a;
    type TA<'a> = &'a FfiTile where Self: 'a;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn tile(&self, x: u32, y: u32) -> Option<Self::TA<'_>> {
        if x >= self.width || y >= self.height {
            return None;
        }

        unsafe {
            let idx = x as usize + (y as usize * self.width as usize);
            Some(self.tiles.add(idx).as_ref().unwrap())
        }
    }
}
