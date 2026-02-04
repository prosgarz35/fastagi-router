use std::io::{self, BufRead, Write};

const MAPPINGS: &[(&str, &str)] = &[
    ("79235253998", "501"), ("73843602313", "501"),
    ("79235254061", "502"), ("73843601773", "502"),
    ("79235254150", "503"),
    ("79235254132", "504"), ("73843602414", "504"),
    ("79235254389", "505"),
    ("79235254439", "506"), ("73843601771", "506"),
    ("79235254667", "507"), ("73843600912", "507"),
    ("79235254706", "508"), ("73843600911", "508"),
    ("79235255049", "509"), ("73843601331", "509"),
    ("79235255136", "510"), ("73843601221", "510"),
    ("79234688941", "511"),
    ("79234689436", "512"),
    ("79234693619", "513"),
    ("79234693746", "514"),
    ("79234693868", "515"),
    ("79234698651", "516"),
    ("79234698906", "517"),
    ("79234702567", "518"),
    ("79235069558", "519"),
    ("79235237068", "520"),
];

fn normalize(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        6 => Some(format!("73843{}", digits)),
        10 => Some(format!("7{}", digits)),
        11 => {
            if digits.starts_with('8') {
                Some(format!("7{}", &digits[1..]))
            } else if digits.starts_with('7') {
                Some(digits)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            break;
        }
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("SET VARIABLE LOOKUP_SUCCESS \"FALSE\"");
        io::stdout().flush()?;
        return Ok(());
    }

    let mode = &args[1];
    let dialed = &args[2];
    let caller = args.get(3).map_or("", String::as_str);

    let mut success = false;

    match mode.as_str() {
        "in" => {
            if let Some(normal) = normalize(dialed) {
                if let Some((_, ext)) = MAPPINGS.iter().find(|&&(n, _)| n == normal) {
                    println!("SET VARIABLE DIAL_TARGET \"{}\"", ext);
                    success = true;
                }
            }
        }
        "out" => {
            if let Some((trunk, _)) = MAPPINGS.iter().find(|&&(n, e)| n == caller || e == caller) {
                if let Some(num) = normalize(dialed) {
                    println!("SET VARIABLE DIAL_TRUNK \"{}\"", trunk);
                    println!("SET VARIABLE DIAL_NUMBER \"{}\"", num);
                    success = true;
                }
            }
        }
        _ => {}
    }

    println!(
        "SET VARIABLE LOOKUP_SUCCESS \"{}\"",
        if success { "TRUE" } else { "FALSE" }
    );
    io::stdout().flush()?;
    Ok(())
}