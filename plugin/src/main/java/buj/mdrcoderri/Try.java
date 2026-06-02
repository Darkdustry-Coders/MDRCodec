package buj.mdrcoderri;

public class Try {
    private Try() {}

    interface TryVoid {
        void get() throws Throwable;
    }
    public static void v(TryVoid run) {
        try {
            run.get();
        } catch (Throwable e) {
            throw new RuntimeException(e);
        }
    }

    interface TryLong {
        long get() throws Throwable;
    }
    public static long l(TryLong run) {
        try {
            return run.get();
        } catch (Throwable e) {
            throw new RuntimeException(e);
        }
    }

    interface TryValue<T> {
        T get() throws Throwable;
    }
    public static <T> T x(TryValue<T> run) {
        try {
            return run.get();
        } catch (Throwable e) {
            throw new RuntimeException(e);
        }
    }

    public static void close(AutoCloseable close) {
        try {
            close.close();
        } catch (Throwable e) {
            throw new RuntimeException(e);
        }
    }
}
