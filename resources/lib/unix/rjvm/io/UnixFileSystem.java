package rjvm.io;

import internal.Extends;
import java.io.File;
import java.io.IOException;

@Extends("java.io.FileSystem")
final class UnixFileSystem{
    private final char slash = '/';
    private final char colon = ':';
    private final String userDir = normalize(System.getProperty("user.dir"));

    public char getSeparator(){
        return slash;
    }

    public char getPathSeparator(){
        return colon;
    }

    /* A normal Unix pathname contains no duplicate slashes and does not end
           with a slash.  It may be the empty string. */

    /**
     * Normalize the given pathname, starting at the given
     * offset; everything before off is already normal, and there's at least
     * one duplicate or trailing slash to be removed
     */
    private String normalize(String pathname, int off) {
        int n = pathname.length();
        while ((n > off) && (pathname.charAt(n - 1) == '/')) n--;
        if (n == 0) return "/";
        if (n == off) return pathname.substring(0, off);

        StringBuilder sb = new StringBuilder(n);
        if (off > 0) sb.append(pathname, 0, off);
        char prevChar = 0;
        for (int i = off; i < n; i++) {
            char c = pathname.charAt(i);
            if ((prevChar == '/') && (c == '/')) continue;
            sb.append(c);
            prevChar = c;
        }
        return sb.toString();
    }

    /* Check that the given pathname is normal.  If not, invoke the real
       normalizer on the part of the pathname that requires normalization.
       This way we iterate through the whole pathname string only once. */
    public String normalize(String pathname) {
        int doubleSlash = pathname.indexOf("//");
        if (doubleSlash >= 0) {
            return normalize(pathname, doubleSlash);
        }
        if (pathname.endsWith("/")) {
            return normalize(pathname, pathname.length() - 1);
        }
        return pathname;
    }

    public int prefixLength(String pathname) {
        return pathname.startsWith("/") ? 1 : 0;
    }

    public String canonicalize(String path) throws IOException {
        return canonicalize0(path);
    }
    private native String canonicalize0(String path) throws IOException;

    /* -- Attribute accessors -- */

    public int getBooleanAttributes(File f) {
        int rv = getBooleanAttributes0(f);
        return rv | isHidden(f);
    }

    private native int getBooleanAttributes0(File f);

    private static int isHidden(File f) {
        //return f.getName().startsWith(".") ? BA_HIDDEN : 0;
        return f.getName().startsWith(".") ? 8 : 0;
    }

    public String getDefaultParent() {
        return "/";
    }

    private static String trimSeparator(String s) {
        int len = s.length();
        if (len > 1 && s.charAt(len - 1) == '/')
            return s.substring(0, len - 1);
        return s;
    }

    public String resolve(String parent, String child) {
        if (child.isEmpty()) return parent;
        if (child.charAt(0) == '/') {
            if (parent.equals("/")) return child;
            return trimSeparator(parent + child);
        }
        if (parent.equals("/")) return trimSeparator(parent + child);
        return trimSeparator(parent + '/' + child);
    }

    /* -- Path operations -- */

    public boolean isAbsolute(File f) {
        return (prefixLength(f.getPath()) != 0);
    }

    public String resolve(File f) {
        if (isAbsolute(f)) return f.getPath();
        return resolve(userDir, f.getPath());
    }

    /* -- Basic infrastructure -- */

    public int compare(File f1, File f2) {
        return f1.getPath().compareTo(f2.getPath());
    }

    public int hashCode(File f) {
        return f.getPath().hashCode() ^ 1234321;
    }
}