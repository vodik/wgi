use std::io::Read;

#[no_mangle]
pub extern "C" fn _start() {
    let length: usize = std::env::var("CONTENT_LENGTH")
        .map(|var| var.parse().unwrap())
        .unwrap_or(0);

    if length != 0 {
        let mut buf = vec![0u8; length];
        std::io::stdin().read_exact(&mut buf).unwrap();

        let input = std::str::from_utf8(&buf).unwrap();
        let markdown = comrak::markdown_to_html(input, &comrak::ComrakOptions::default());
        let output = format!(
            r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
</head>
<body>
{}
</body>
"#,
            markdown
        );

        println!("Content-Type: text/html");
        println!("Content-Length: {}", output.len());
        println!();
        print!("{}", output);
    }
}
