use std::{fs::File, slice};

use crate::{data::ChangeKind, ffi::data::FfiWorld, io::RawFileHandle, sync::SeekingEncoder};

type Encoder = SeekingEncoder<File>;

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_new(fd: RawFileHandle) -> *mut Encoder {
    unsafe {
        Box::leak(Box::new(SeekingEncoder::new(fd.into_file(), {
            #[cfg(feature = "lz4")]
            {
                crate::opt::Compression::Lz4 { mode: lz4::block::CompressionMode::HIGHCOMPRESSION(10) }
            }
            #[cfg(not(feature = "lz4"))]
            {
                crate::opt::Compression::None
            }
        }).unwrap()))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_map(this: *mut Encoder, data: *const FfiWorld) {
    unsafe {
        let this = this.as_mut().unwrap();
        let data = data.as_ref().unwrap();
        this.write_map(data).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_map_raw(this: *mut Encoder, data: *const u8, len: usize) {
    unsafe {
        let this = this.as_mut().unwrap();
        let data = data.as_ref().unwrap();
        this.write_map_raw(slice::from_raw_parts(data, len)).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_id_raw(this: *mut Encoder, data: *const u8, len: usize) {
    unsafe {
        let this = this.as_mut().unwrap();
        let data = data.as_ref().unwrap();
        this.write_id_raw(slice::from_raw_parts(data, len)).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_mod_unit_moved(this: *mut Encoder, unit_id: i32, x: f32, y: f32) {
    unsafe {
        let this = this.as_mut().unwrap();
        this.write_change(&ChangeKind::UnitMoved { unit_id, x, y }).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_mod_unit_rot(this: *mut Encoder, unit_id: i32, rot: u8) {
    unsafe {
        let this = this.as_mut().unwrap();
        this.write_change(&ChangeKind::UnitRotation { unit_id, rot }).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_mod_unit_dead(this: *mut Encoder, unit_id: i32) {
    unsafe {
        let this = this.as_mut().unwrap();
        this.write_change(&ChangeKind::UnitDead { unit_id }).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_write_mod_unit_despawn(this: *mut Encoder, unit_id: i32) {
    unsafe {
        let this = this.as_mut().unwrap();
        this.write_change(&ChangeKind::UnitDespawn { unit_id }).unwrap();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_drop(ptr: *mut Encoder) {
    unsafe { Box::from_raw(ptr).flush().expect("flush failed"); }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mdrcoder_basic_encoder_flush(ptr: *mut Encoder) {
    unsafe { ptr.as_mut().unwrap().flush().expect("flush failed"); }
}
