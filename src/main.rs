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
    ("79234688941", "511"), ("79234689436", "512"),
    ("79234693619", "513"), ("79234693746", "514"),
    ("79234693868", "515"), ("79234698651", "516"),
    ("79234698906", "517"), ("79234702567", "518"),
    ("79235069558", "519"), ("79235237068", "520"),
];

fn main() {
    for l in io::stdin().lock().lines() {
        if l.map_or(true, |s| s.trim().is_empty()) { break; }
    }

    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    
    let digits: String = args.next().unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();

    let dialed = match digits.len() {
        6 => format!("73843{digits}"),
        10 => format!("7{digits}"),
        11 if digits.starts_with('8') => format!("7{}", &digits[1..]),
        _ => digits
    };

    let caller = args.next().unwrap_or_default();

    let found = if mode == "in" {
        MAPPINGS.iter()
            .find(|&&(n, _)| n == dialed)
            .map(|&(_, ext)| {
                println!(r#"SET VARIABLE DIAL_TARGET "{ext}""#);
                true
            })
            .unwrap_or(false)
    } else {
        MAPPINGS.iter()
            .find(|&&(t, e)| t == caller || e == caller)
            .map(|&(trunk, _)| {
                println!(r#"SET VARIABLE DIAL_TRUNK "{trunk}""#);
                println!(r#"SET VARIABLE DIAL_NUMBER "{dialed}""#);
                true
            })
            .unwrap_or(false)
    };

    println!(r#"SET VARIABLE LOOKUP_SUCCESS "{}""#, if found { "TRUE" } else { "FALSE" });
    io::stdout().flush().ok();
}