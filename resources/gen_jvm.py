import re

def transform_type(typ):
    if typ in ["jint", "jobject", "jintArray", "jobjectArray", "jboolean", "jlong", "jclass", "jbyteArray", "jstring", "jvalue", "jdouble", "jfieldID", "jmethodID", "jsize"]:
        return typ
    elif typ == "int":
        return "i32"
    elif typ == "void":
        return "c_void"
    elif typ == "long":
        return "c_long"
    elif typ in ["const char *", "char *", "char*"]:
        return "*const c_char"
    elif typ in ["void*", "void *"]:
        return "*const c_void"
    elif typ in ["JNIEnv *", "JNIEnv*"]:
        return "*mut JNIEnv"
    elif typ == "size_t":
        return "isize"
    elif typ == "unsigned char":
        return "c_uchar"
    elif typ == "JVM_DTraceProvider*":
        return "*mut JVM_DTraceProvider"
    elif typ == "JVM_ExceptionTableEntryType *":
        return "*mut JVM_ExceptionTableEntryType"
    elif typ == "const jbyte *":
        return "*const jbyte"
    elif typ == "unsigned char *":
        return "*const c_uchar"
    elif typ == "unsigned short *":
        return "*const c_ushort"
    elif typ == "jlong *":
        return "*const jlong"
    elif typ == "jint *":
        return "*const jint"
    elif typ == "int *":
        return "*const c_int"
    elif typ == "struct sockaddr *":
        return "*const sockaddr"
    elif typ == "jvm_version_info*":
        return "*const jvm_version_info"
    else:
        print(typ)

def main():
    with open("jvm.h") as f:
        content = f.read()
        patt = re.compile(r"JNIEXPORT\s+([\w*\s]+)\s+JNICALL\s*(\w+)\(([\w*\s,]+)\);")
        arg_patt = re.compile(r"(\w[\w\s]*[\s*]+)(\w+)")

        output = ""

        for m in patt.finditer(content):
            ret, name, args = m.groups()

            output += "#[unsafe(no_mangle)]\n"

            args_rust = ""
            for mm in arg_patt.finditer(args):
                typ, arg_name = mm.groups()
                if arg_name.strip() == "type":
                    arg_name = "typ"
                typ_rust = transform_type(typ.strip())
                args_rust += f'{arg_name}: {typ_rust}, '
            args_rust = args_rust[:-2]

            ret_rust = transform_type(ret)
            output += f'pub extern "C" fn {name}({args_rust}) -> {ret_rust}{{\n'
            output += "    unimplemented!();\n}\n\n"

        with open("jvm.rs", "w") as f:
            f.write(output)

if __name__ == "__main__":
    main()