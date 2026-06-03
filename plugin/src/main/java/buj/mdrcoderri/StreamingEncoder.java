package buj.mdrcoderri;

import java.nio.ByteOrder;
import java.util.WeakHashMap;

import arc.struct.Seq;
import arc.util.Log;
import mindustry.Vars;
import mindustry.ctype.MappableContent;
import mindustry.gen.Unit;

/**
 * Encoder with 'Seek' support.
 */
public class StreamingEncoder implements AutoCloseable {
    private final long encoder;
    private final LibMdrCodec lib;

    StreamingEncoder(long encoder, LibMdrCodec lib) {
        this.encoder = encoder;
        this.lib = lib;

        writeIdSnapshot(Vars.content.blocks());
        writeIdSnapshot(Vars.content.items());
        writeIdSnapshot(Vars.content.units());
        writeIdSnapshot(Vars.content.unitCommands());
        writeIdSnapshot(Vars.content.unitStances());
        writeIdSnapshot(Vars.content.items());
        writeIdSnapshot(Vars.content.liquids());
        writeIdSnapshot(Vars.content.weathers());
        writeMapSnapshot();
    }

    private <T extends MappableContent> long idSnapshotLength(Seq<T> content) {
        var length = new CalcLength();

        length.addByte();

        content.each(x -> {
            length.addShort();
            length.addByte();
            var name = x.name.getBytes(Vars.charset);
            assert name.length < 256;
            length.length += name.length;
        });

        return length.length;
    }
    private <T extends MappableContent> void writeIdSnapshot(Seq<T> content) {
        long len = idSnapshotLength(content);
        long address = LibC.malloc(len);
        final var buf = LibC.directBuffer(address, (int) len).order(ByteOrder.LITTLE_ENDIAN);

        buf.put((byte) content.first().getContentType().ordinal());

        content.each(x -> {
            buf.putShort(x.id);
            var name = x.name.getBytes(Vars.charset);
            buf.put((byte) name.length);
            buf.put(name);
        });

        Try.v(() -> Main.libmdrcodec.mdrcoderBasicEncoderWriteIdRaw(encoder, address, len));
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

    private static class UnitCache {
        float x;
        float y;
        byte dir;
    }
    private final WeakHashMap<Unit, UnitCache> cache = new WeakHashMap<>();

    private byte dirByte(float dir) {
        dir /= 360;
        dir %= 1;
        if (dir < 0) dir++;
        dir *= 256;
        return (byte) (int) dir;
    }

    public void unitUpdate(Unit unit) {
        Try.v(() -> {
            var cache = this.cache.get(unit);
            if (cache == null) {
                cache = new UnitCache();
                cache.x = unit.x;
                cache.y = unit.y;
                cache.dir = dirByte(unit.rotation);
                this.cache.put(unit, cache);
            }
            var dx = cache.x - unit.x;
            if (dx < 0) dx = -dx;
            var dy = cache.y - unit.y;
            if (dy < 0) dy = -dy;
            if (dx > 4 || dy > 4) {
                lib.mdrcoderBasicEncoderWriteModUnitMoved(encoder, unit.id, unit.x, unit.y);
                cache.x = unit.x;
                cache.y = unit.y;
            }
            var dir = dirByte(unit.rotation);
            if (cache.dir != dir) {
                lib.mdrcoderBasicEncoderWriteModUnitRot(encoder, unit.id, dir);
                cache.dir = dir;
            }
        });
    }

    public void unitDied(Unit unit) {
        Try.v(() -> lib.mdrcoderBasicEncoderWriteModUnitDead(encoder, unit.id));
    }

    public void unitDespawned(Unit unit) {
        Try.v(() -> lib.mdrcoderBasicEncoderWriteModUnitDespawn(encoder, unit.id));
    }

    public void flush() {
        Try.v(() -> lib.mdrcoderBasicEncoderFlush(encoder));
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
