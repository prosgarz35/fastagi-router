use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use phf::phf_map;

static NUMBER_TO_EXT: phf::Map<&'static str, &'static str> = phf_map! {
    "79235253998"=>"501","79235254061"=>"502","79235254150"=>"503","79235254132"=>"504",
    "79235254389"=>"505","79235254439"=>"506","79235254667"=>"507","79235254706"=>"508",
    "79235255049"=>"509","79235255136"=>"510","73843602313"=>"501","73843601773"=>"502",
    "73843731773"=>"502","73843602414"=>"504","73843601771"=>"506","73843600912"=>"507",
    "73843600911"=>"508","73843731458"=>"508","73843601331"=>"509","73843731313"=>"509",
    "73843601221"=>"510","73843731500"=>"510","104"=>"501","135"=>"502","119"=>"502",
    "111"=>"508","106"=>"509"
};

static EXT_TO_TRUNK: phf::Map<&'static str, &'static str> = phf_map! {
    "501"=>"79235253998","502"=>"79235254061","503"=>"79235254150","504"=>"79235254132",
    "505"=>"79235254389","506"=>"79235254439","507"=>"79235254667","508"=>"79235254706",
    "509"=>"79235255049","510"=>"79235255136"
};

fn set_var(name: &str, value: &str) {
    println!("SET VARIABLE {} \"{}\"", name, value);
    let _ = io::stdout().flush();
}

fn set_failure_with_reason(reason: &str) {
    set_var("LOOKUP_SUCCESS", "FALSE");
    set_var("IS_INTERNAL_DEST", "FALSE");
    set_var("DIAL_TARGET", "");
    set_var("LOOKUP_REASON", reason);
}

fn set_success_internal(target: &str) {
    set_var("LOOKUP_SUCCESS", "TRUE");
    set_var("IS_INTERNAL_DEST", "TRUE");
    set_var("DIAL_TARGET", target);
}

fn set_success_external(target: &str) {
    set_var("LOOKUP_SUCCESS", "TRUE");
    set_var("IS_INTERNAL_DEST", "FALSE");
    set_var("DIAL_TARGET", target);
}

fn sanitize(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

fn normalize_number(dial_s: &str) -> Option<String> {
    match dial_s.len() {
        3 => Some(dial_s.to_string()),
        6 => Some(format!("73843{}", dial_s)),
        11 => match dial_s.chars().next() {
            Some('8') => Some(format!("7{}", &dial_s[1..])),
            Some('7') => Some(dial_s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn main() -> io::Result<()> {
    let mut vars = HashMap::new();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let l = line?;
        if l.trim().is_empty() { break; }
        if let Some((k, v)) = l.split_once(':') {
            vars.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let dialed = vars.get("agi_arg_1").map(|s| s.as_str()).unwrap_or("");
    let caller = vars.get("agi_arg_2").map(|s| s.as_str()).unwrap_or("");
    let mode = vars.get("agi_arg_3").map(|s| s.as_str()).unwrap_or("outbound");

    if mode != "inbound" && mode != "outbound" {
        set_failure_with_reason("invalid_mode");
        return Ok(());
    }

    let dial_s = sanitize(dialed);
    let caller_s = sanitize(caller);

    if dial_s.is_empty() {
        set_failure_with_reason("empty_dial");
        return Ok(());
    }

    if mode == "inbound" {
        if let Some(&ext) = NUMBER_TO_EXT.get(dial_s.as_str()) {
            set_success_internal(ext);
        } else {
            set_failure_with_reason("unknown_inbound_did");
        }
        return Ok(());
    }
    
    if let Some(&t) = EXT_TO_TRUNK.get(caller_s.as_str()) {
        set_var("DIAL_TRUNK", t);
    }

    let normalized = match normalize_number(&dial_s) {
        Some(n) => n,
        None => {
            set_failure_with_reason("normalize_failed_wrong_length");
            return Ok(());
        }
    };

    if let Some(&ext) = NUMBER_TO_EXT.get(&normalized) {
        set_success_internal(ext);
    } else {
        if dial_s.len() == 3 {
            set_failure_with_reason("short_internal_rejected");
        } else {
            set_success_external(&normalized);
        }
    }

    Ok(())
}