use std::{fs::File, io::BufWriter, slice};

use crate::{ffi::data::FfiWorld, io::RawFileHandle, sync::SeekingEncoder};

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
unsafe extern "C" fn mdrcoder_basic_encoder_drop(ptr: *mut Encoder) {
    unsafe { Box::from_raw(ptr).flush().expect("flush failed"); }
}
