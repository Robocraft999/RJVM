use crate::class_file::methods::descriptor::MethodDescriptor;

pub fn native_escape(name: &str) -> String {
    let mut escaped = String::new();
    for c in name.chars(){
        match c{
            'A'..='Z' | 'a'..='z' | '0'..='9' => escaped.push(c),
            '/' => escaped.push('_'),
            '_' => escaped.push_str("_1"),
            ';' => escaped.push_str("_2"),
            '[' => escaped.push_str("_3"),
            other => escaped.push_str(format!("_0{:04x}", other as u16).as_str()),
        }
    }
    escaped
}

pub fn native_escaped_descriptor(descriptor: &MethodDescriptor) -> String {
    let mut escaped = String::new();
    for ft in descriptor.args.iter().chain(descriptor.return_type.iter()) {
        escaped.push_str(native_escape(ft.to_descriptor().as_str()).as_str());
    }
    escaped
}


#[cfg(test)]
mod tests{
    use crate::class_file::field_info::{native_escape, native_escaped_descriptor};
    use crate::class_file::methods::descriptor::MethodDescriptor;

    #[test]
    fn test_native_escape() {
        assert_eq!("", native_escape(""));
        assert_eq!("A", native_escape("A"));
        assert_eq!("hello_test_Test123", native_escape("hello/test/Test123"));
        assert_eq!("hello_Pr_000fcfer", native_escape("hello/Prüfer"))
    }

    #[test]
    fn test_native_escape_descriptor(){
        let descriptor = MethodDescriptor::new(String::from("(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;I)Ljava/lang/Object;"));
        assert_eq!("Ljava_lang_reflect_Constructor_2_3Ljava_lang_Object_2ILjava_lang_Object_2", native_escaped_descriptor(&descriptor));
    }

    fn class_and_method_escaped(class_name: &str, method_name: &str, descriptor: &MethodDescriptor) -> (String, String) {
        let mut short = String::from("Java_");
        short.push_str(native_escape(class_name).as_str());
        short.push('_');
        short.push_str(native_escape(method_name).as_str());

        let mut long = short.clone();
        long.push_str("__");
        long.push_str(native_escaped_descriptor(&descriptor).as_str());

        (short, long)
    }

    #[test]
    fn test_native_escaped_class_and_method_1(){
        let class_name = "sun/reflect/NativeConstructorAccessorImpl";
        let method_name = "newInstance0";
        let descriptor = MethodDescriptor::new(String::from("(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;I)Ljava/lang/Object;"));
        let expected = (
            String::from("Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0"),
            String::from("Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0__Ljava_lang_reflect_Constructor_2_3Ljava_lang_Object_2ILjava_lang_Object_2")
        );
        assert_eq!(expected, class_and_method_escaped(class_name, method_name, &descriptor));
    }
    #[test]
    fn test_native_escaped_class_and_method_2(){
        //sun/awt/X11GraphicsEnvironment.getNumScreens()I
        let class_name = "sun/awt/X11GraphicsEnvironment";
        let method_name = "getNumScreens";
        let descriptor = MethodDescriptor::new(String::from("()I"));
        let expected = (
            String::from("Java_sun_awt_X11GraphicsEnvironment_getNumScreens"),
            String::from("Java_sun_awt_X11GraphicsEnvironment_getNumScreens__I")
        );
        assert_eq!(expected, class_and_method_escaped(class_name, method_name, &descriptor));
    }
}