use std::io::{self, BufRead, Write};

const REGION_PREFIX: &str = "73843";

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

enum AgiMode {
    Incoming,
    Outgoing,
    Invalid,
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

fn finish_agi(stdout: &mut io::Stdout, success: bool) {
    let status = if success { "TRUE" } else { "FALSE" };
    let _ = writeln!(stdout, r#"SET VARIABLE LOOKUP_SUCCESS "{}""#, status);
    let _ = stdout.flush();
}

fn main() {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        match line {
            Ok(l) if l.trim().is_empty() => break,
            Err(_) => return,
            _ => continue,
        }
    }

    let mut args = std::env::args().skip(1);
    let mode_raw = args.next().map(|s| s.trim().to_lowercase()).unwrap_or_default();
    
    let dialed_arg = args.next().unwrap_or_default();
    let dialed_raw = dialed_arg.trim();
    
    let caller_arg = args.next().unwrap_or_default();
    let caller_raw = caller_arg.trim();

    let mode = match mode_raw.as_str() {
        "in" => AgiMode::Incoming,
        "out" => AgiMode::Outgoing,
        _ => AgiMode::Invalid,
    };

    let dialed = normalize(dialed_raw);
    let caller = normalize(caller_raw);

    if let AgiMode::Invalid = mode {
        finish_agi(&mut stdout, false);
        return;
    }

    if dialed.len() < 6 || dialed.len() > 15 || !dialed.starts_with('7') {
        finish_agi(&mut stdout, false);
        return;
    }

    let found = match mode {
        AgiMode::Incoming => {
            MAPPINGS.iter()
                .find(|(trunk, _)| *trunk == dialed)
                .map(|(_, ext)| {
                    let _ = writeln!(stdout, r#"SET VARIABLE DIAL_TARGET "{}""#, ext);
                })
                .is_some()
        }
        AgiMode::Outgoing if !caller.is_empty() => {
            MAPPINGS.iter()
                .find(|(trunk, ext)| *trunk == caller || *ext == caller)
                .map(|(trunk, _)| {
                    let _ = writeln!(stdout, r#"SET VARIABLE DIAL_TRUNK "{}""#, trunk);
                    let _ = writeln!(stdout, r#"SET VARIABLE DIAL_NUMBER "{}""#, dialed);
                })
                .is_some()
        }
        _ => false,
    };

    finish_agi(&mut stdout, found);
}