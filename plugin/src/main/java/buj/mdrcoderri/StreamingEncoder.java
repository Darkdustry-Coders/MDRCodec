package buj.mdrcoderri;

import java.nio.ByteOrder;

import mindustry.Vars;

/**
 * Encoder with 'Seek' support.
 */
public class StreamingEncoder implements AutoCloseable {
    private final long encoder;
    private final LibMdrCodec lib;

    StreamingEncoder(long encoder, LibMdrCodec lib) {
        this.encoder = encoder;
        this.lib = lib;
    }

    long mapSnapshotLength() {
        var length = new CalcLength();
        length.addInt(); // width
        length.addInt(); // height

        Vars.world.tiles.eachTile(tile -> {
            length.addShort(); // block
            length.addShort(); // floor
            length.addShort(); // overlay
            length.addByte();  // block data
            length.addByte();  // floor data
            length.addByte();  // overlay data
            length.addInt();   // extra data
            length.addByte();  // building
        });

        return length.length;
    }
    public void writeMapSnapshot() {
        long len = mapSnapshotLength();
        long address = LibC.malloc(len);
        final var buf = LibC.directBuffer(address, (int) len).order(ByteOrder.LITTLE_ENDIAN);

        buf.putInt(Vars.world.width());
        buf.putInt(Vars.world.height());

        Vars.world.tiles.eachTile(tile -> {
            buf.putShort(tile.blockID());
            buf.putShort(tile.floorID());
            buf.putShort(tile.overlayID());
            buf.put(tile.data);
            buf.put(tile.floorData);
            buf.put(tile.overlayData);
            buf.putInt(tile.extraData);
            buf.put((byte) 0);
        });

        Try.v(() -> Main.libmdrcodec.mdrcoderBasicEncoderWriteMapRaw(encoder, address, len));
    }

    @Override
    public void close() throws Exception {
        try {
            lib.mdrcoderBasicEncoderDrop(encoder);
        } catch (Throwable t) {
            throw new RuntimeException(t);
        }
    }
}
