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
    Some(match d.len() {
        6  => format!("73843{d}"),
        10 => format!("7{d}"),
        11 if d.starts_with('8') => format!("7{}", &d[1..]),
        11 if d.starts_with('7') => d,
        _  => return None,
    })
}

fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        if line.ok().map_or(true, |s| s.trim().is_empty()) { break; }
    }

    let args: Vec<_> = std::env::args().collect();
    let [_, mode, dialed, caller @ ..] = args.as_slice() else {
        println!(r#"SET VARIABLE LOOKUP_SUCCESS "FALSE""#);
        return Ok(());
    };

    let caller = caller.first().map_or("", String::as_str);
    let success = match mode.as_str() {
        "in" => normalize(dialed)
            .and_then(|n| MAPPINGS.iter().find(|&&(num,_)| num == n))
            .map(|&(_, ext)| { println!(r#"SET VARIABLE DIAL_TARGET "{ext}""#); true }),

        "out" => MAPPINGS.iter().find(|&&(n,e)| n == caller || e == caller)
            .and_then(|&(trunk,_)| normalize(dialed).map(|num| {
                println!(r#"SET VARIABLE DIAL_TRUNK "{trunk}""#);
                println!(r#"SET VARIABLE DIAL_NUMBER "{num}""#);
                true
            })),

        _ => None,
    }.unwrap_or(false);

    println!(r#"SET VARIABLE LOOKUP_SUCCESS "{}""#, if success { "TRUE" } else { "FALSE" });
    io::stdout().flush()
}