import java.io.OutputStream;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;
import java.lang.Class;

public class ClassPath {

    public static void main(String[] args) throws FileNotFoundException, IOException, ClassNotFoundException {
        String path = args[0];
        String basename = path.replaceAll("\\w+\\.", "");
        System.out.println(basename);
        OutputStream o = new FileOutputStream(path.replaceAll("\\.", "/") + ".class");
        Class.forName(args[0]).getResourceAsStream(basename + ".class")
            .transferTo(o);
        System.out.println();
    }
}