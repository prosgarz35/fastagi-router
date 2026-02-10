use std::io::{self, BufRead, Write};

const REGION_PREFIX: &str = "73843";
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

fn send_cmd(stdout: &mut io::Stdout, args: std::fmt::Arguments<'_>) {
    stdout.write_fmt(args).ok();
    stdout.write_all(b"\n").ok();
    stdout.flush().ok();
}

fn normalize(s: &str) -> String {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    match d.as_str() {
        s if s.len() == 11 && s.starts_with('8') => format!("7{}", &s[1..]),
        s if s.len() == 11 && s.starts_with('7') => s.to_string(),
        s if s.len() == 10 => format!("7{}", s),
        s if s.len() == 6 => format!("{}{}", REGION_PREFIX, s),
        _ => d,
    }
}

fn main() {
    let mut stdout = io::stdout();
    
    for line in io::stdin().lock().lines() {
        if let Ok(l) = line {
            if l.trim().is_empty() { break; }
        } else { return; }
    }

    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default().to_lowercase();
    let dialed = normalize(&args.next().unwrap_or_default());
    let caller = normalize(&args.next().unwrap_or_default());

    if dialed.len() < 6 || dialed.len() > 15 || !dialed.starts_with('7') {
        send_cmd(&mut stdout, format_args!("SET VARIABLE LOOKUP_SUCCESS \"FALSE\""));
        return;
    }

    let found = match mode.as_str() {
        "in" => {
            MAPPINGS.iter()
                .find(|(trunk, _)| *trunk == dialed)
                .map(|(_, ext)| {
                    send_cmd(&mut stdout, format_args!("SET VARIABLE DIAL_TARGET \"{}\"", ext));
                }).is_some()
        },
        "out" if !caller.is_empty() => {
            MAPPINGS.iter()
                .find(|(trunk, ext)| *trunk == caller || *ext == caller)
                .map(|(trunk, _)| {
                    send_cmd(&mut stdout, format_args!("SET VARIABLE DIAL_TRUNK \"{}\"", trunk));
                    send_cmd(&mut stdout, format_args!("SET VARIABLE DIAL_NUMBER \"{}\"", dialed));
                }).is_some()
        },
        _ => false,
    };

    send_cmd(&mut stdout, format_args!("SET VARIABLE LOOKUP_SUCCESS \"{}\"", if found { "TRUE" } else { "FALSE" }));
}