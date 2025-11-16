fn main() {
    println!(">{}", env!("LD_LIBRARY_PATH"));
    println!("{:?}", std::env::var("PROJECT_DIR"));
    jvm::run();
}
