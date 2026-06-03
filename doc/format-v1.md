# MDR format version 1

An early draft. World chunks are WIP.

```bytedoc
(byte[] { 'M', 'D', 'R', '\0' }: magic)
(u16le: format version = 1)
(CompressionSettings)
(BackBound)
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

## A full 0 border to stop any further reading.
BackBound:
    (u8 = 0)
    (u32le = 0)
    (u64le = 0)
    (u64le = 0)

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

MapChunkv1: (Chunk.{ body = Compressed<MapChunkv1Body>, kind = 2 })
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
    (u8(data_block))
    (u8(data_floor))
    (u8(data_overlay))
    (u32le(data_extra))
    (u8(has_building) = 0: reserved)
Unitv1:
    (u32(id))
    (u16le(type))
    (f32le(x))
    (f32le(y))
    (u8(rotation): rotation in 256-edge degrees)

IdChunkv1: (Chunk.{ body = Compressed<IdChunkv1Body>, kind = 3 })
IdChunkv1Body:
    (u8: content type, see [[Content Types]])
    (IdChunkv1Record[])
IdChunkv1Record:
    (u16le(id): recorded id)
    (u8(strlen))
    (byte[strlen](name): record name)

ModChunkv1: (Chunk.{ body = Compressed<ModChunkv1Body>, kind = 4 })
ModChunkv1Body:
    (ModChunkv1Record[])
ModChunkv1Record:
    (u32(dts): duration since the previous change or record timestamp)
    (u8(kind): change id)
    {
        if kind == 0,
            << noop, no data >>
            ,
        if kind == 1,
            << unit moved >>
            (i32(unit_id))
            (f32le(x))
            (f32le(y))
            ,
        if kind == 2,
            << unit rotation changed >>
            (i32(unit_id))
            (u8(rotation))
            ,
        if kind == 3,
            << unit dead >>
            (i32(unit_id))
            ,
        if kind == 4,
            << unit despawn >>
            (i32(unit_id))
            ,
        else (unreachable)
    }

## Compressed content.
##
## The actual contents depend on compression settings.
Compressed<T>:
    (byte[]: compressed contents)
```

Typically a file would have a structure of
```bytedoc
(byte[] { 'M', 'D', 'R', '\0' }: magic)
(u16le: format version = 1)
(CompressionSettings)
(JmpChunkv1)
(IdChunkv1)
(IdChunkv1)
...
(IdChunkv1)
(MapChunkv1)
...
```

## Content Types
- 0: item
- 1: block
- 6: unit type
