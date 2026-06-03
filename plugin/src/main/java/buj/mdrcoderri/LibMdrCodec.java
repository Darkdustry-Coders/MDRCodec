package buj.mdrcoderri;

import java.io.RandomAccessFile;
import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodType;

import arc.util.Reflect;
import jdk.incubator.foreign.CLinker;
import jdk.incubator.foreign.FunctionDescriptor;
import jdk.incubator.foreign.SymbolLookup;

public class LibMdrCodec {
    private final MethodHandle mdrcoderBasicEncoderNew;
    private final MethodHandle mdrcoderBasicEncoderWriteMap;
    private final MethodHandle mdrcoderBasicEncoderWriteMapRaw;
    private final MethodHandle mdrcoderBasicEncoderWriteIdRaw;
    private final MethodHandle mdrcoderBasicEncoderWriteModUnitMoved;
    private final MethodHandle mdrcoderBasicEncoderWriteModUnitRot;
    private final MethodHandle mdrcoderBasicEncoderWriteModUnitDead;
    private final MethodHandle mdrcoderBasicEncoderWriteModUnitDespawn;
    private final MethodHandle mdrcoderBasicEncoderFlush;
    private final MethodHandle mdrcoderBasicEncoderDrop;

    public LibMdrCodec() throws UnsatisfiedLinkError {
        System.loadLibrary("mdrcodec");
        mdrcoderBasicEncoderNew = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_new").orElseThrow(),
                MethodType.methodType(long.class, int.class),
                FunctionDescriptor.of(CLinker.C_LONG, CLinker.C_INT));
        mdrcoderBasicEncoderWriteMap = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_map").orElseThrow(),
                MethodType.methodType(void.class, long.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_LONG));
        mdrcoderBasicEncoderWriteMapRaw = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_map_raw").orElseThrow(),
                MethodType.methodType(void.class, long.class, long.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_LONG, CLinker.C_LONG));
        mdrcoderBasicEncoderWriteIdRaw = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_id_raw").orElseThrow(),
                MethodType.methodType(void.class, long.class, long.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_LONG, CLinker.C_LONG));
        mdrcoderBasicEncoderWriteModUnitMoved = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_mod_unit_moved").orElseThrow(),
                MethodType.methodType(void.class, long.class, int.class, float.class, float.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_INT, CLinker.C_FLOAT, CLinker.C_FLOAT));
        mdrcoderBasicEncoderWriteModUnitRot = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_mod_unit_rot").orElseThrow(),
                MethodType.methodType(void.class, long.class, int.class, byte.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_INT, CLinker.C_CHAR));
        mdrcoderBasicEncoderWriteModUnitDead = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_mod_unit_dead").orElseThrow(),
                MethodType.methodType(void.class, long.class, int.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_INT));
        mdrcoderBasicEncoderWriteModUnitDespawn = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_write_mod_unit_despawn").orElseThrow(),
                MethodType.methodType(void.class, long.class, int.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG, CLinker.C_INT));
        mdrcoderBasicEncoderFlush = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_flush").orElseThrow(),
                MethodType.methodType(void.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG));
        mdrcoderBasicEncoderDrop = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("mdrcoder_basic_encoder_drop").orElseThrow(),
                MethodType.methodType(void.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG));
    }

    /**
     * Create a new encoder.
     */
    public StreamingEncoder startRecording(RandomAccessFile file) throws Throwable {
        int fd = Reflect.get(file.getFD(), "fd");
        return new StreamingEncoder((long) mdrcoderBasicEncoderNew.invokeWithArguments(fd), this);
    }

    /**
     * Write a MAP chunk.
     */
    void mdrcoderBasicEncoderWriteMap(long encoder, long world) throws Throwable {
        mdrcoderBasicEncoderWriteMap.invokeWithArguments(encoder, world);
    }

    /**
     * Write a raw MAP chunk.
     * <p>
     * Data in the provided slice must be a valid MAP chunk, otherwise the file may
     * get corrupted.
     */
    void mdrcoderBasicEncoderWriteMapRaw(long encoder, long data, long length) throws Throwable {
        mdrcoderBasicEncoderWriteMapRaw.invokeWithArguments(encoder, data, length);
    }

    /**
     * Write a raw ID chunk.
     * <p>
     * Data in the provided slice must be a valid ID chunk, otherwise the file may
     * get corrupted.
     */
    void mdrcoderBasicEncoderWriteIdRaw(long encoder, long data, long length) throws Throwable {
        mdrcoderBasicEncoderWriteIdRaw.invokeWithArguments(encoder, data, length);
    }

    /**
     * Append unit position change.
     * <p>
     * This will create a mod chunk once another chunk will need to be
     * written, a write timeout has expired, or the writer is flushed,
     * or the modifications buffer is full.
     */
    void mdrcoderBasicEncoderWriteModUnitMoved(long encoder, int unitId, float x, float y) throws Throwable {
        mdrcoderBasicEncoderWriteModUnitMoved.invokeWithArguments(encoder, unitId, x, y);
    }

    /**
     * Append unit rotation change.
     * <p>
     * This will create a mod chunk once another chunk will need to be
     * written, a write timeout has expired, or the writer is flushed,
     * or the modifications buffer is full.
     */
    void mdrcoderBasicEncoderWriteModUnitRot(long encoder, int unitId, byte rot) throws Throwable {
        mdrcoderBasicEncoderWriteModUnitRot.invokeWithArguments(encoder, unitId, rot);
    }

    /**
     * Append unit death.
     * <p>
     * This will create a mod chunk once another chunk will need to be
     * written, a write timeout has expired, or the writer is flushed,
     * or the modifications buffer is full.
     */
    void mdrcoderBasicEncoderWriteModUnitDead(long encoder, int unitId) throws Throwable {
        mdrcoderBasicEncoderWriteModUnitDead.invokeWithArguments(encoder, unitId);
    }

    /**
     * Append unit despawn.
     * <p>
     * This will create a mod chunk once another chunk will need to be
     * written, a write timeout has expired, or the writer is flushed,
     * or the modifications buffer is full.
     */
    void mdrcoderBasicEncoderWriteModUnitDespawn(long encoder, int unitId) throws Throwable {
        mdrcoderBasicEncoderWriteModUnitDespawn.invokeWithArguments(encoder, unitId);
    }

    /**
     * Flush encoder.
     * <p>
     * This will create a new MOD chunk if any changes were submitted.
     */
    void mdrcoderBasicEncoderFlush(long encoder) throws Throwable {
        mdrcoderBasicEncoderFlush.invokeWithArguments(encoder);
    }

    /**
     * Destroy encoder.
     */
    void mdrcoderBasicEncoderDrop(long encoder) throws Throwable {
        mdrcoderBasicEncoderDrop.invokeWithArguments(encoder);
    }
}
