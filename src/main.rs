use std::io::{self, BufRead, Write};

const MAPPINGS: &[(&str, &str)] = &[
    ("79235253998", "501"), ("73843602313", "501"),
    ("79235254061", "502"), ("73843601773", "502"),
    ("79235254150", "503"), ("79235254132", "504"),
    ("73843602414", "504"), ("79235254389", "505"),
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

fn send_cmd(out: &mut impl Write, name: &str, value: &str) -> io::Result<()> {
    writeln!(out, "SET VARIABLE {} \"{}\"", name, value)?;
    out.flush()
}

fn normalize(s: &str) -> String {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    match d.len() {
        6 => {
            let mut n = String::with_capacity(11);
            n.push_str("73843");
            n.push_str(&d);
            n
        }
        10 => {
            let mut n = String::with_capacity(11);
            n.push('7');
            n.push_str(&d);
            n
        }
        11 if d.starts_with('8') => {
            let mut n = String::with_capacity(11);
            n.push('7');
            n.push_str(&d[1..]);
            n
        }
        11 => d,
        _ => d,
    }
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
    let caller_raw = args.next().unwrap_or_default();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if dialed_raw.is_empty() {
        return send_cmd(&mut out, "LOOKUP_SUCCESS", "FALSE");
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines().flatten() {
        if line.trim().is_empty() { break; }
    }

    let dialed = normalize(&dialed_raw);
    let caller = normalize(&caller_raw);

    if dialed.len() != 11 || !dialed.starts_with('7') {
        return send_cmd(&mut out, "LOOKUP_SUCCESS", "FALSE");
    }

    let mut found = false;

    match mode.as_str() {
        "in" => {
            if let Some((_, ext)) = MAPPINGS.iter().find(|(num, _)| num == dialed) {
                send_cmd(&mut out, "DIAL_TARGET", ext)?;
                found = true;
            }
        }
        "out" if !caller.is_empty() => {
            if let Some((trunk, _)) = MAPPINGS.iter().find(|(num, ext)| num == caller || ext == caller) {
                send_cmd(&mut out, "DIAL_TRUNK", trunk)?;
                send_cmd(&mut out, "DIAL_NUMBER", &dialed)?;
                found = true;
            }
        }
        _ => {}
    }

    send_cmd(&mut out, "LOOKUP_SUCCESS", if found { "TRUE" } else { "FALSE" })
}