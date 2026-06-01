# MDR format version 1

An early draft. World chunks are WIP.

```bytedoc
(byte[] { 'M', 'D', 'R', '\0' }: magic)
(u16le: format version = 1)
(CompressionSettings)
(Chunk<dyn>[?]: data..)

CompressionSettings:
    (u8(discriminator): compression kind)
    {
        if discriminator == 0,
            << no options >>
            ,
        if discriminator == 1,
            << lz4 >>
            (u8(lz4_mode))
            {
                if lz4_mode == 0,
                    << default mode, no options >>
                    ,
                if lz4_mode == 1,
                    << fast compression >>
                    (i32le: lz4 quality)
                    ,
                if lz4_mode == 2,
                    << high compression >>
                    (i32le: lz4 quality)
                    ,
                else (unreachable)
            }
            ,
        if discriminator == 2,
            << deflate compression >>
            (u32le: compression level)
            ,
        if discriminator == 3,
            << zlib compression >>
            (u32le: compression level)
            ,
        if discriminator == 4,
            << gzip compression >>
            (u32le: compression level)
            ,
        else (unreachable)
    }

Chunk:
    (u8(kind): chunk kind)
    (u64le(timestamp): timestamp in milliseconds)
    (u32le(len): stored body length in bytes)
    (byte[len](body): chunk body, possibly compressed)
    (u8 = kind)
    (u32le = len)
    (u64le = timestamp)
    (u64le: file pointer to a previous JMP chunk. 0 if none)

JmpChunkv1: (Chunk.{ body = JmpChunkv1Body, kind = 1 })
JmpChunkv1Body:
    (JmpChunkv1Record[]: jump records until the end of chunk body, uncompressed)
## A Record.
##
## If validity byte is not 1, all the other bytes are ignored.
##
## By default all entries are zeroes.
JmpChunkv1Record:
    (u8(kind): chunk kind)
    (u64le: chunk timestamp)
    (u64le: file pointer)
    (u8(validity))

MapChunkv1: (Chunk.{ body = MapChunkv1Body, kind = 2 })
MapChunkv1Body:
    (u32le(width))
    (u32le(height))
    (Tilev1[width * height](tiles))
    (u32le(unit_count))
    (Unitv1[unit_count](units))
Tilev1:
    (u16le(block))
    (u16le(floor))
    (u16le(overlay))
    (u8(team))
    (u8(data_block))
    (u8(data_floor))
    (u8(data_overlay))
    (u32le(data_extra))
    (u8(has_building) = 0: reserved)
Unitv1:
    (u16le(type))
    (f32le(x))
    (f32le(y))
    (u8(rotation): rotation in 256-edge degrees)

IdChunkv1: (Chunk.{ body = IdChunkv1Body, kind = 3 })
IdChunkv1Body:
    (IdChunkv1Record[])
IdChunkv1Record:
    (u16le(id): recorded id)
    (u16le(strlen))
    (byte[strlen](name): record name)
```

Typically a file would have a structure of
```bytedoc
(byte[] { 'M', 'D', 'R', '\0' }: magic)
(u16le: format version = 1)
(CompressionSettings)
(JmpChunkv1)
(IdChunkv1)
(MapChunkv1)
...
```
