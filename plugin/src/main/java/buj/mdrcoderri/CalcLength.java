package buj.mdrcoderri;

import arc.func.Cons;

public class CalcLength {
    public long length = 0;

    public CalcLength addByte() {
        length++;
        return this;
    }

    public CalcLength addShort() {
        length += 2;
        return this;
    }

    public CalcLength addInt() {
        length += 4;
        return this;
    }

    public CalcLength addPtr() {
        return addLong();
    }
    public CalcLength addLong() {
        length += 8;
        return this;
    }

    public CalcLength times(long count, Cons<CalcLength> what) {
        CalcLength obj = new CalcLength();
        what.get(obj);
        length += obj.length * count;
        return this;
    }
}
