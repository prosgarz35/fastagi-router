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
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    match d.len() {
        6  => Some(format!("73843{}", d)),
        10 => Some(format!("7{}", d)),
        11 => match d.as_bytes()[0] {
            b'7' => Some(d),
            b'8' => Some(format!("7{}", &d[1..])),
            _    => None,
        },
        _  => None,
    }
}

fn main() -> io::Result<()> {
    let stdin = io::stdin().lock();
    for line in stdin.lines().flatten() {
        if line.trim().is_empty() { break; }
    }

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 { return Ok(()); }

    let mode   = &args[1];
    let dialed = &args[2];
    let caller = args.get(3).map_or("", String::as_str);

    let mut out = io::stdout().lock();

    let success = match mode.as_str() {
        "in" => normalize(dialed)
            .and_then(|n| MAPPINGS.iter().find(|&&(num, _)| num == n.as_str()))
            .map(|&(_, ext)| {
                let _ = writeln!(out, "SET VARIABLE DIAL_TARGET \"{}\"", ext);
                true
            })
            .unwrap_or(false),

        "out" => MAPPINGS.iter()
            .find(|&&(n, e)| n == caller || e == caller)
            .and_then(|&(trunk, _)| normalize(dialed).map(|num| {
                let _ = writeln!(out, "SET VARIABLE DIAL_TRUNK \"{}\"", trunk);
                let _ = writeln!(out, "SET VARIABLE DIAL_NUMBER \"{}\"", num);
                true
            }))
            .unwrap_or(false),

        _ => false,
    };

    let _ = writeln!(out, "SET VARIABLE LOOKUP_SUCCESS \"{}\"", if success { "TRUE" } else { "FALSE" });
    out.flush()
}