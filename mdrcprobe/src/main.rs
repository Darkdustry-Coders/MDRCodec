use std::{fs::File, io::BufReader, path::Path, process::exit};

use mdrcodec::{data::{ChangeKind, ChunkBody}, sync::StreamingDecoder};

fn main() {
    let Some(file) = std::env::args_os().nth(1) else {
        println!("usage: mdrcprobe <file>");
        exit(1);
    };
    let path = Path::new(&file);
    let file = BufReader::new(match File::open(path) {
        Ok(x) => x,
        Err(why) => {
            println!("mdrcprobe: cannot open {:?}: {why:#}", path.display());
            exit(1);
        }
    });
    let codec = match StreamingDecoder::new(file) {
        Ok(x) => x,
        Err(why) => {
            println!("mdrcprobe: failed to read header: {why:#}");
            exit(1);
        }
    };
    for (i, frame) in codec.enumerate() {
        let frame = match frame {
            Ok(x) => x,
            Err(why) => {
                println!("mdrcprobe: invalid frame {i}: {why}");
                exit(1);
            }
        };
        println!("Chunk {i}: {}", frame.kind());
        match &frame.body {
            ChunkBody::Mapv1(x) => {
                println!("- Size: {}x{}", x.width(), x.height());
            },
            ChunkBody::Idv1(x) => {
                println!("- Content Type: {}", x.content_type());
                println!("- Registered: {}", x.entries().count());
            }
            ChunkBody::Modv1(x) => {
                for change in x.entries() {
                    match change.kind {
                        ChangeKind::UnitMoved { unit_id, x, y } => {
                            println!("- At {}ms: unit moved id={unit_id}, x={}, y={}", (change.offset + frame.timestamp).as_millis(), x.round() as i64, y.round() as i64);
                        },
                        ChangeKind::UnitRotation { unit_id, rot } => {
                            println!("- At {}ms: unit rotation id={unit_id}, rot={}", (change.offset + frame.timestamp).as_millis(), rot);
                        },
                        ChangeKind::UnitDead { unit_id } => {
                            println!("- At {}ms: unit dead id={unit_id}", (change.offset + frame.timestamp).as_millis());
                        },
                        ChangeKind::UnitDespawn { unit_id } => {
                            println!("- At {}ms: unit despawn id={unit_id}", (change.offset + frame.timestamp).as_millis());
                        },
                    }
                }
            }
        }
    }
}
