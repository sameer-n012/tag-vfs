package util;

import java.nio.ByteBuffer;

public class Conversion {

    public static int ushtoi(short sh) { return sh & 0xffff; }

    public static short itoush(int i) { return (short) (i & 0xffff); }

    public static int mxui() { return 0xffffffff; }

    public static short mxush() { return (short) 0xffff; }

    public static int batoi(byte[] bytes) {
        return ((bytes[0] & 0xFF) << 24) |
                ((bytes[1] & 0xFF) << 16) |
                ((bytes[2] & 0xFF) << 8 ) |
                ((bytes[3] & 0xFF) << 0 );
    }

    public static short batosh(byte[] bytes) {
        return (short) (((bytes[0] & 0xFF) << 8 ) |
                ((bytes[1] & 0xFF) << 0 ));
    }

    public static short batob(byte[] bytes) {
        return bytes[0];
    }


    public static byte[] itoba(int value) {
        return new byte[] {
                (byte) (value >>> 24),
                (byte) (value >>> 16),
                (byte) (value >>> 8),
                (byte) (value >>> 0)
        };
    }

    public static byte[] ltoba(long value, int n) {
        byte[] b = new byte[n];
        for(int i = 0; i < n; i++) {
            b[i] = (byte) (value >>> 8*(n-i-1));
        }
        return b;
    }

    public static long batol(byte[] arr, int n) {

        ByteBuffer bb = ByteBuffer.allocate(8);

        if(n < 8) {
            byte[] pref = new byte[8-n];
            for (int i = n; i < 8; i++) {
                pref[0] = 0;
            }
            bb.put(pref);
        }
        bb.put(arr);
        return bb.getLong();
    }

    public static int toUnsigned(short s) {
        return ((int) s) - Short.MIN_VALUE;
    }

    public static long toUnsigned(int i) {
        return ((long) i) - Integer.MIN_VALUE;
    }
}
