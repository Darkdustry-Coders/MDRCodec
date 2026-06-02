package buj.mdrcoderri;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodType;
import java.nio.ByteBuffer;

import arc.func.Prov;
import arc.util.pooling.Pool;
import arc.util.pooling.Pools;
import jdk.incubator.foreign.CLinker;
import jdk.incubator.foreign.FunctionDescriptor;
import jdk.incubator.foreign.SymbolLookup;

public class LibC {
    private LibC() {}
    private static final MethodHandle malloc;
    private static final MethodHandle realloc;
    private static final MethodHandle free;


    static {
        malloc = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("malloc").orElseThrow(),
                MethodType.methodType(long.class, long.class),
                FunctionDescriptor.of(CLinker.C_LONG, CLinker.C_LONG));
        realloc = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("realloc").orElseThrow(),
                MethodType.methodType(long.class, long.class, long.class),
                FunctionDescriptor.of(CLinker.C_LONG, CLinker.C_LONG, CLinker.C_LONG));
        free = Main.link.downcallHandle(
                SymbolLookup.loaderLookup().lookup("free").orElseThrow(),
                MethodType.methodType(void.class, long.class),
                FunctionDescriptor.ofVoid(CLinker.C_LONG));
    }

    public static class AutoFreeAlloc implements AutoCloseable {
        private static final Pool<AutoFreeAlloc> pool = Pools.get(AutoFreeAlloc.class, AutoFreeAlloc.prov);
        private static final Prov<AutoFreeAlloc> prov = AutoFreeAlloc::new;

        public long ptr;
        AutoFreeAlloc() {}

        @Override
        public void close() {
            free(ptr);
            pool.free(this);
        }

        public static AutoFreeAlloc autoFreeAlloc(long length) {
            var x = pool.obtain();
            x.ptr = malloc(length);
            return x;
        }
    }

    public static AutoFreeAlloc alloc(long length) {
        return AutoFreeAlloc.autoFreeAlloc(length);
    }

    public static long malloc(long length) {
        var x = Try.l(() -> (long) malloc.invokeWithArguments(length));
        if (x == 0) throw new OutOfMemoryError("Could not allocate "+length+" bytes");
        return x;
    }

    public static long realloc(long ptr, long length) {
        var x = Try.l(() -> (long) realloc.invokeWithArguments(ptr, length));
        if (x == 0) throw new OutOfMemoryError("Could not allocate "+length+" bytes");
        return x;
    }

    public static void realloc(LongRef ptr, long length) {
        var x = Try.l(() -> (long) realloc.invokeWithArguments(ptr, length));
        if (x == 0) throw new OutOfMemoryError("Could not allocate "+length+" bytes");
        ptr.r = x;
    }

    public static void free(long length) {
        Try.v(() -> free.invokeWithArguments(length));
    }

    public static ByteBuffer directBuffer(long ptr, int length) {
        return Try.x(() -> {
            var bbc = Class.forName("java.nio.DirectByteBuffer");
            var constr = bbc.getDeclaredConstructor(long.class, int.class);
            constr.setAccessible(true);
            return (ByteBuffer) constr.newInstance(ptr, length);
        });
    }
}
