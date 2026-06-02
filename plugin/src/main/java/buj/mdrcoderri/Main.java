package buj.mdrcoderri;

import java.io.RandomAccessFile;
import java.time.Instant;
import java.util.Date;

import arc.Events;
import jdk.incubator.foreign.CLinker;
import mindustry.Vars;
import mindustry.game.EventType.PlayEvent;
import mindustry.game.EventType.ResetEvent;
import mindustry.mod.Plugin;

public class Main extends Plugin {
    public static final CLinker link = CLinker.getInstance();
    public static LibMdrCodec libmdrcodec;
    public static StreamingEncoder encoder;

    String alignTo(String s, int count) {
        if (s.length() >= count) return s;

        var b = new StringBuilder();
        while (b.length() + s.length() < count) b.append('0');
        return b.append(s).toString();
    }

    @Override
    public void init() {
        try {
            libmdrcodec = new LibMdrCodec();
        } catch (Exception error) {
            throw new RuntimeException("Loading of MDRCodec failed. Please ensure that `libmdrcodec.so`/`mdrcodec.dll` is located in library path.", error);
        }

        Events.on(PlayEvent.class, event -> {
            Date date = Date.from(Instant.now());
            // It may be deprecated, but I don't care.
            int year = date.getYear() + 1900;

            var str = alignTo(Integer.toString(year), 4) + "-" + alignTo(Integer.toString(date.getMonth()), 2) +
                "-" + alignTo(Integer.toString(date.getDate()), 2) + "_" + alignTo(Integer.toString(date.getHours()), 2) +
                ":" + alignTo(Integer.toString(date.getMinutes()), 2) + ":" + alignTo(Integer.toString(date.getSeconds()), 2)
                + ".mdr";

            final var fi = new Ref<>(Vars.dataDirectory.child("records"));
            if (!fi.r.exists()) fi.r.mkdirs();
            fi.r = fi.r.child(str);
            encoder = Try.x(() -> libmdrcodec.openEncoder(new RandomAccessFile(fi.r.file(), "rw")));
            encoder.writeMapSnapshot();
        });

        Events.on(ResetEvent.class, event -> {
            if (encoder != null) {
                Try.close(encoder);
                encoder = null;
            }
        });
    }
}
