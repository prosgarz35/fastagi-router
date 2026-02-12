use std::io::{self, BufRead, Write};

const MAPPINGS: &[(&str, &str)] = &[
    ("79235253998", "501"),
    ("79235254061", "502"),
    ("79235254150", "503"),
    ("79235254132", "504"),
    ("79235254389", "505"),
    ("79235254439", "506"),
    ("79235254667", "507"),
    ("79235254706", "508"),
    ("79235255049", "509"),
    ("79235255136", "510"),
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

fn normalize(num: &str) -> Option<String> {
    let digits: String = num.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        6 => Some(format!("73843{}", digits)),
        11 if digits.starts_with('8') => Some(format!("7{}", &digits[1..])),
        11 if digits.starts_with('7') => Some(digits),
        _ => None,
    }
}

fn send(out: &mut impl Write, name: &str, value: &str) -> io::Result<()> {
    writeln!(out, "SET VARIABLE {} \"{}\"", name, value)?;
    out.flush()
}

fn main() {
    if let Err(_) = run() {
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default().to_lowercase();
    let dialed_raw = args.next().unwrap_or_default();
    let caller_raw = args.next().unwrap_or_default().trim().to_string();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if dialed_raw.is_empty() {
        return send(&mut out, "LOOKUP_SUCCESS", "FALSE");
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines().flatten() {
        if line.trim().is_empty() { break; }
    }

    let mut found = false;

    if let Some(dialed) = normalize(&dialed_raw) {
        match mode.as_str() {
            "in" => {
                if let Some((_, ext)) = MAPPINGS.iter().find(|&(did, _)| *did == dialed) {
                    send(&mut out, "DIAL_TARGET", ext)?;
                    found = true;
                }
            }
            "out" if !caller_raw.is_empty() => {
                if let Some((did, _)) = MAPPINGS.iter().find(|&(_, ext)| *ext == caller_raw) {
                    send(&mut out, "DIAL_TRUNK", did)?;
                    send(&mut out, "DIAL_NUMBER", &dialed)?;
                    found = true;
                }
            }
            _ => {}
        }
    }

    send(&mut out, "LOOKUP_SUCCESS", if found { "TRUE" } else { "FALSE" })
}