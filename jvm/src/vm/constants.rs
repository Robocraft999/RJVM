#![allow(non_upper_case_globals)]
#![allow(unused)]
// FIELD INDICES

// java.io.FileInputStream
pub const FILEINPUTSTREAM_path_INDEX: usize = 2;

// java.io.File
pub const FILE_path_INDEX: usize = 1;

// java.lang.Class
pub const CLASS_name_INDEX: usize = 5;

// java.lang.ClassLoader$NativeLibrary
pub const CLASSLOADER_NATIVELIBRARY_handle_INDEX: usize = 0;
pub const CLASSLOADER_NATIVELIBRARY_name_INDEX: usize = 3;
pub const CLASSLOADER_NATIVELIBRARY_loaded_INDEX: usize = 5;

// java.lang.Long
pub const LONG_value_INDEX: usize = 4;

// java.lang.String
pub const STRING_value_INDEX: usize = 0;
pub const STRING_hash_INDEX: usize = 1;

// java.lang.System
pub const SYSTEM_in_INDEX: usize = 0;
pub const SYSTEM_out_INDEX: usize = 1;
pub const SYSTEM_err_INDEX: usize = 2;

// java.lang.Thread
pub const THREAD_name_INDEX: usize = 0;
pub const THREAD_priority_INDEX: usize = 1;
pub const THREAD_eetop_INDEX: usize = 3;
pub const THREAD_stillborn_INDEX: usize = 6;
pub const THREAD_target_INDEX: usize = 7;
pub const THREAD_group_INDEX: usize = 8;
pub const THREAD_threadStatus_INDEX: usize = 18;

// java.lang.ThreadGroup
pub const THREADGROUP_parent_INDEX: usize = 0;
pub const THREADGROUP_name_INDEX: usize = 1;
pub const THREADGROUP_maxPriority_INDEX: usize = 2;
pub const THREADGROUP_nUnstartedThreads_INDEX: usize = 6;

// java.lang.Throwable
pub const THROWABLE_detailsMessage_INDEX: usize = 2;

// java.lang.invoke.LambdaForm
pub const LAMBDAFORM_vmentry_INDEX: usize = 6;

// java.lang.invoke.MemberName
pub const MEMBERNAME_clazz_INDEX: usize = 0;
pub const MEMBERNAME_name_INDEX: usize = 1;
pub const MEMBERNAME_type_INDEX: usize = 2;
pub const MEMBERNAME_flags_INDEX: usize = 3;

// java.lang.invoke.MethodHandle
pub const METHODHANDLE_type_INDEX: usize = 0;
pub const METHODHANDLE_form_INDEX: usize = 1;

// java.lang.invoke.MethodType
pub const METHODTYPE_rtype_INDEX: usize = 1;
pub const METHODTYPE_ptypes_INDEX: usize = 2;

// java.lang.reflect.Constructor
pub const CONSTRUCTOR_clazz_INDEX: usize = 7;
pub const CONSTRUCTOR_slot_INDEX: usize = 8;
pub const CONSTRUCTOR_parameterTypes_INDEX: usize = 9;
pub const CONSTRUCTOR_exceptionTypes_INDEX: usize = 10;
pub const CONSTRUCTOR_modifiers_INDEX: usize = 11;

// java.lang.reflect.Method
pub const METHOD_clazz_INDEX: usize = 7;
pub const METHOD_slot_INDEX: usize = 8;
pub const METHOD_name_INDEX: usize = 9;
pub const METHOD_returnType_INDEX: usize = 10;
pub const METHOD_parameterTypes_INDEX: usize = 11;
pub const METHOD_exceptionTypes_INDEX: usize = 12;
pub const METHOD_modifiers_INDEX: usize = 13;

// java.lang.reflect.Field
pub const FIELD_clazz_INDEX: usize = 4;
pub const FIELD_name_INDEX: usize = 6;
pub const FIELD_type_INDEX: usize = 7;
pub const FIELD_modifiers_INDEX: usize = 8;

// CLASS NAMES
pub mod classes {
    pub const JAVA_IO_FILE_INPUT_STREAM: &str = "java/io/FileInputStream";
    pub const JAVA_IO_FILE_OUTPUT_STREAM: &str = "java/io/FileOutputStream";
    pub const JAVA_IO_UNIX_FILE_SYSTEM: &str = "java/io/UnixFileSystem";
    pub const JAVA_LANG_CLASS: &str = "java/lang/Class";
    pub const JAVA_LANG_CLASSLOADER: &str = "java/lang/ClassLoader";
    pub const JAVA_LANG_DOUBLE: &str = "java/lang/Double";
    pub const JAVA_LANG_FLOAT: &str = "java/lang/Float";
    pub const JAVA_LANG_INVOKE_METHOD_HANDLE: &str = "java/lang/invoke/MethodHandle";
    pub const JAVA_LANG_INVOKE_MHN: &str = "java/lang/invoke/MethodHandleNatives";
    pub const JAVA_LANG_INVOKE_METHOD_TYPE: &str = "java/lang/invoke/MethodType";
    pub const JAVA_LANG_LONG: &str = "java/lang/Long";
    pub const JAVA_LANG_OBJECT: &str = "java/lang/Object";
    pub const JAVA_LANG_OBJECT_ARR: &str = "[Ljava/lang/Object;";
    pub const JAVA_LANG_PROCESS_ENVIRONMENT: &str = "java/lang/ProcessEnvironment";
    pub const JAVA_LANG_REFLECT_CONSTRUCTOR: &str = "java/lang/reflect/Constructor";
    pub const JAVA_LANG_REFLECT_FIELD: &str = "java/lang/reflect/Field";
    pub const JAVA_LANG_REFLECT_METHOD: &str = "java/lang/reflect/Method";
    pub const JAVA_LANG_RUNTIME: &str = "java/lang/Runtime";
    pub const JAVA_LANG_STRING: &str = "java/lang/String";
    pub const JAVA_LANG_SYSTEM: &str = "java/lang/System";
    pub const JAVA_LANG_THREAD: &str = "java/lang/Thread";
    pub const JAVA_LANG_THREAD_GROUP: &str = "java/lang/ThreadGroup";
    pub const JAVA_LANG_THROWABLE: &str = "java/lang/Throwable";
    pub const SUN_MISC_PERF: &str = "sun/misc/Perf";
    pub const SUN_MISC_SIGNAL: &str = "sun/misc/Signal";
    pub const SUN_MISC_UNSAFE: &str = "sun/misc/Unsafe";
    pub const SUN_MISC_URL_CLASSPATH: &str = "sun/misc/URLClassPath";
    pub const SUN_MISC_VM: &str = "sun/misc/VM";
    pub const SUN_NIO_FS_UND: &str = "sun/nio/fs/UnixNativeDispatcher";
    pub const SUN_REFLECT_REFLECTION: &str = "sun/reflect/Reflection";
    pub const SUN_REFLECT_NCAI: &str = "sun/reflect/NativeConstructorAccessorImpl";
    pub const SUN_REFLECT_NMAI: &str = "sun/reflect/NativeMethodAccessorImpl";
}