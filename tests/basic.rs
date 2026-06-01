use std::io::Cursor;

use mdrcodec::enc::Encoder;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

fn tokio() -> Runtime {
    RuntimeBuilder::new_current_thread().build().unwrap()
}

#[test]
fn encoder_can_open_file() {
    let mut vfile = vec![];
    let mut enc = Encoder::builder(Cursor::new(&mut vfile))
        .writeable()
        .seekable()
        .build()
        .expect("failed to build encoder");
    assert_eq!(vfile.as_slice(), b"MDR\0");
}

#[test]
fn encoder_can_open_file_async() {
    tokio().block_on(async {
        let mut vfile = vec![];
        let mut enc = Encoder::builder(futures::io::Cursor::new(&mut vfile))
            .async_seekable()
            .async_writeable()
            .build_async()
            .await
            .expect("failed to build encoder");
        assert_eq!(vfile.as_slice(), b"MDR\0");
    });
}
