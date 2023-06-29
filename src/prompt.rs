use anyhow::Context;
use inquire::*;

pub(crate) fn account_prompt() -> anyhow::Result<(String, String)> {
    println!("Please enter your email and password to log in ChatGPT!");
    let email = Text::new("Email:")
        .with_validator(required!("email is required"))
        .with_validator(min_length!(8))
        .with_help_message("OpenAI account email, Format: example@gmail.com")
        .prompt()?;

    let password = Password::new("Password:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_validator(required!("password is required"))
        .with_validator(min_length!(8))
        .with_formatter(&|_| String::from("Input received"))
        .with_help_message("OpenAI account password")
        .with_custom_confirmation_error_message("The password don't match.")
        .prompt()
        .context("An error happened when asking for your account, try again later.")?;
    Ok((email, password))
}
