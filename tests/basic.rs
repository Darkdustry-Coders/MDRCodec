use std::io::Cursor;

use futures::io::Cursor as AsyncCursor;
use mdrcodec::{future::AsyncSeekingEncoder, opt::Compression, sync::SeekingEncoder};
use tokio::test as atest;

#[test]
fn encoder_can_open_file() {
    let mut vfile = vec![];
    let mut enc = SeekingEncoder::new(Cursor::new(&mut vfile), Compression::None)
        .expect("failed to build encoder");
    assert_eq!(vfile.as_slice(), b"MDR\0\x01\0\0");
}

#[atest(flavor = "current_thread")]
async fn encoder_can_open_file_async() {
    let mut vfile = vec![];
    let mut enc = AsyncSeekingEncoder::new(AsyncCursor::new(&mut vfile), Compression::None)
        .await
        .expect("failed to build encoder");
    assert_eq!(vfile.as_slice(), b"MDR\0\x01\0\0");
}
