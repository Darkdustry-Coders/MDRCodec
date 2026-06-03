# .mdr codec

A codec for Mindurka's replay format.

> [!WARNING]
> Hazmat suit on!
>
> This project is incredible WIP and the project in this state may not even work!

## Performance notes

While we don't strictly enforce it, we assume that all I/O is buffered, and thus an individual `write`
call to write one byte is okay. If not, writing could be extremely slow.

This *may be changed in the future*.

## Planned features

- Support for both sync and async
- C API for linking with Java via Project Pamana
- Maybe an in-tree player, idk
- Tracking plans, mouse positions, etc

## `unsafe` policy

If it makes things less annoying and you've checked beforehand, use it, especially if there's a performance
benefit.
