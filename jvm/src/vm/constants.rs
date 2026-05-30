#![allow(non_upper_case_globals)]
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
pub const SYSTEM_out_INDEX: usize = 1;
pub const SYSTEM_err_INDEX: usize = 2;

// java.lang.Thread
pub const THREAD_name_INDEX: usize = 0;
pub const THREAD_priority_INDEX: usize = 1;
pub const THREAD_stillborn_INDEX: usize = 6;
pub const THREAD_group_INDEX: usize = 8;

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