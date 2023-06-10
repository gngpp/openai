use tokio::io::AsyncWriteExt;

pub async fn print_stream(
    out: &mut tokio::io::Stdout,
    previous_message: String,
    message: String,
) -> anyhow::Result<String> {
    if message.starts_with(&*previous_message) {
        let new_chars: String = message.chars().skip(previous_message.len()).collect();
        out.write_all(new_chars.as_bytes()).await?;
    } else {
        out.write_all(message.as_bytes()).await?;
    }
    out.flush().await?;
    Ok(message)
}
