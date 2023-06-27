#[tokio::main]
async fn main() {
    let x = openai::arkose::murmur::murmurhash3_x64_128(b"test", 31);
    // ff55565a476832ed3409c64597508ca4
    println!("{:x}{:x}", x.0, x.1);
    let x = openai::arkose::crypto::encrypt("Hello, World", "my_secret_key");
    println!("{}", x);

    let x = openai::arkose::ArkoseToken::new("gpt4-sb").await.unwrap();
    println!("{:?}", x);

    let x = openai::arkose::ArkoseToken::new("gpt4-sb").await.unwrap();
    println!("{:?}", x)
}
