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
     *
     * Data in the provided slice must be a valid MAP chunk, otherwise the file may
     * get corrupted.
     */
    void mdrcoderBasicEncoderWriteMapRaw(long encoder, long data, long length) throws Throwable {
        mdrcoderBasicEncoderWriteMapRaw.invokeWithArguments(encoder, data, length);
    }

    /**
     * Write a raw ID chunk.
     *
     * Data in the provided slice must be a valid ID chunk, otherwise the file may
     * get corrupted.
     */
    void mdrcoderBasicEncoderWriteIdRaw(long encoder, long data, long length) throws Throwable {
        mdrcoderBasicEncoderWriteIdRaw.invokeWithArguments(encoder, data, length);
    }

    /**
     * Destroy encoder.
     */
    void mdrcoderBasicEncoderDrop(long encoder) throws Throwable {
        mdrcoderBasicEncoderDrop.invokeWithArguments(encoder);
    }
}
