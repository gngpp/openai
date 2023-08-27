fn find_single_consecutive_char(s: &str, target: char) -> Result<String, &'static str> {
    let mut count = 0;
    let mut chars = String::new();
    let mut found = None;

    for c in s.chars() {
        if c == target {
            count += 1;
            chars.push(target);
        } else {
            if count > 0 {
                // 如果已经找到一组连续的目标字符
                if found.is_some() {
                    return Err("Found more than one group of consecutive target characters");
                }
                found = Some(chars.clone());
            }
            count = 0;
            chars.clear();
        }
    }

    // 最后检查，以防字符串以连续的目标字符结尾
    if count > 0 {
        if found.is_some() {
            return Err("Found more than one group of consecutive target characters");
        }
        found = Some(chars);
    }

    found.ok_or("No consecutive target characters found")
}

fn main() {
    let test_str1 = "herickh2@hotmail.com----Herick@26";
    let test_str2 = "herickh2@hotmail.com----Herick@26--";
    let target_char = '-';

    match find_single_consecutive_char(test_str1, target_char) {
        Ok(chars) => println!(
            "Successfully found a single group of consecutive target characters: {}",
            chars
        ),
        Err(e) => println!("Error: {}", e),
    }

    match find_single_consecutive_char(test_str2, target_char) {
        Ok(chars) => println!(
            "Successfully found a single group of consecutive target characters: {}",
            chars
        ),
        Err(e) => println!("Error: {}", e),
    }
}
