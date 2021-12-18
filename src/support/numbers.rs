pub fn num_word(n: u8) -> String {
    match n {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        _ => format!("{}th", n),
    }
}
