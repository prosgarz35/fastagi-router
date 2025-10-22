use std::collections::HashMap;
use std::io::{self, BufRead};
use phf::phf_map;

static NUMBER_TO_EXT: phf::Map<&str, &str> = phf_map! {
    "79235253998"=>"501","79235254061"=>"502","79235254150"=>"503","79235254132"=>"504",
    "79235254389"=>"505","79235254439"=>"506","79235254667"=>"507","79235254706"=>"508",
    "79235255049"=>"509","79235255136"=>"510","73843602313"=>"501","73843601773"=>"502",
    "73843731773"=>"502","73843602414"=>"504","73843601771"=>"506","73843600912"=>"507",
    "73843600911"=>"508","73843731458"=>"508","73843601331"=>"509","73843731313"=>"509",
    "73843601221"=>"510","73843731500"=>"510","104"=>"501","135"=>"502","119"=>"502",
    "111"=>"508","106"=>"509"
};

static EXT_TO_TRUNK: phf::Map<&str, &str> = phf_map! {
    "501"=>"79235253998","502"=>"79235254061","503"=>"79235254150","504"=>"79235254132",
    "505"=>"79235254389","506"=>"79235254439","507"=>"79235254667","508"=>"79235254706",
    "509"=>"79235255049","510"=>"79235255136"
};

fn main() {
    let mut vars = HashMap::new();
    let stdin = io::stdin();

    for line in stdin.lines() {
        match line {
            Ok(l) if l.trim().is_empty() => break,
            Ok(l) => if let Some((k, v)) = l.split_once(':') {
                vars.insert(k.trim().to_string(), v.trim().to_string());
            },
            _ => break,
        }
    }

    let dialed = vars.get("agi_arg_1").map(|s| s.as_str()).unwrap_or("");
    let caller = vars.get("agi_arg_2").map(|s| s.as_str()).unwrap_or("");
    let mode = vars.get("agi_arg_3").map(|s| s.as_str()).unwrap_or("outbound");

    if dialed.is_empty() {
        println!("SET VARIABLE LOOKUP_SUCCESS FALSE");
        return;
    }

    let sanitize = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    let dial_s = sanitize(dialed);
    let caller_s = sanitize(caller);

    if mode == "inbound" {
        if let Some(&ext) = NUMBER_TO_EXT.get(dial_s.as_str()) {
            println!("SET VARIABLE LOOKUP_SUCCESS TRUE");
            println!("SET VARIABLE IS_INTERNAL_DEST TRUE");
            println!("SET VARIABLE DIAL_TARGET {}", ext);
        } else {
            println!("SET VARIABLE LOOKUP_SUCCESS FALSE");
        }
        return;
    }

    if let Some(&t) = EXT_TO_TRUNK.get(caller_s.as_str()) {
        println!("SET VARIABLE DIAL_TRUNK {}", t);
    }

    let (is_internal, target) = if dial_s.len() == 3 {
        NUMBER_TO_EXT.get(dial_s.as_str())
            .map_or((false, dial_s.as_str()), |&ext| (true, ext))
    } else {
        let normalized = match dial_s.len() {
            6 => format!("73843{}", dial_s),
            11 if dial_s.starts_with('8') => format!("7{}", &dial_s[1..]),
            _ => dial_s.clone(),
        };
        NUMBER_TO_EXT.get(normalized.as_str())
            .map_or((false, normalized.as_str()), |&ext| (true, ext))
    };

    if is_internal {
        println!("SET VARIABLE LOOKUP_SUCCESS TRUE");
        println!("SET VARIABLE IS_INTERNAL_DEST TRUE");
        println!("SET VARIABLE DIAL_TARGET {}", target);
    } else if dial_s.len() == 3 {
        println!("SET VARIABLE LOOKUP_SUCCESS FALSE");
    } else {
        println!("SET VARIABLE LOOKUP_SUCCESS TRUE");
        println!("SET VARIABLE IS_INTERNAL_DEST FALSE");
        println!("SET VARIABLE DIAL_TARGET {}", target);
    }
}