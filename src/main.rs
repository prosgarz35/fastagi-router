use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use itoa;
use once_cell::sync::Lazy;

const FEDERAL_PREFIX: u64 = 7;
const CITY_PREFIX_U64: u64 = 73843;
const CITY_MULTIPLIER: u64 = 1_000_000;
const TEN_BILLION: u64 = 10_000_000_000;
const ROUTE_STATUS: &str = "ROUTE_STATUS";
const IS_INTERNAL_DEST: &str = "IS_INTERNAL_DEST";
const TARGET_EXT: &str = "TARGET_EXT";
const OUT_NUMBER: &str = "OUT_NUMBER";
const DIAL_TRUNK: &str = "DIAL_TRUNK";
const VERBOSE_LEVEL_WARNING: u8 = 1;
const VERBOSE_LEVEL_SUCCESS: u8 = 3;

#[derive(Debug)]
enum RouteTarget { Internal(u16), External(u64) }

#[derive(Debug, PartialEq, Clone, Copy)]
enum CallType { Inbound, OldShort, Normalized }

struct AgiResponse {
    status: &'static str,
    is_internal_dest: &'static str,
    target: Option<RouteTarget>,
    outbound_trunk: Option<u64>,
}

static INBOUND_MAP: Lazy<HashMap<u64, u16>> = Lazy::new(|| {
    HashMap::from([
        (79235253998, 501), (73843602313, 501), (79235254061, 502), (73843601773, 502), (73843731773, 502),
        (79235254150, 503), (79235254132, 504), (73843602414, 504), (79235254389, 505), (79235254439, 506),
        (73843601771, 506), (79235254667, 507), (73843600912, 507), (79235254706, 508), (73843600911, 508),
        (73843731458, 508), (79235255049, 509), (73843601331, 509), (73843731313, 509), (79235255136, 510),
        (73843601221, 510), (73843731500, 510),
    ])
});

static OUTBOUND_TRUNK_MAP: Lazy<HashMap<u16, u64>> = Lazy::new(|| {
    HashMap::from([
        (501, 79235253998), (502, 79235254061), (503, 79235254150), (504, 79235254132), (505, 79235254389),
        (506, 79235254439), (507, 79235254667), (508, 79235254706), (509, 79235255049), (510, 79235255136),
    ])
});

static SHORT_CODE_MAP: Lazy<HashMap<u16, u16>> = Lazy::new(|| {
    HashMap::from([
        (104, 501), (135, 502), (119, 502), (111, 508), (106, 509),
    ])
});

fn normalize_number(raw: &str) -> Option<u64> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { return None; }
    let num = digits.parse::<u64>().ok()?;
    match digits.len() {
        11 => match digits.chars().next()? {
            '7' => Some(num),
            '8' => Some(FEDERAL_PREFIX * TEN_BILLION + (num % TEN_BILLION)),
            _ => None,
        },
        10 => Some(FEDERAL_PREFIX * TEN_BILLION + num),
        6 => Some(CITY_PREFIX_U64 * CITY_MULTIPLIER + num),
        _ => None,
    }
}

fn route_inbound(trunk_number: u64) -> Option<RouteTarget> {
    INBOUND_MAP.get(&trunk_number).copied().map(RouteTarget::Internal)
}

fn route_outbound_short(short_code: u16) -> Option<RouteTarget> {
    SHORT_CODE_MAP.get(&short_code).copied().map(RouteTarget::Internal)
}

fn route_normalized(num: u64) -> RouteTarget {
    if let Some(&ext) = INBOUND_MAP.get(&num) { RouteTarget::Internal(ext) }
    else { RouteTarget::External(num) }
}

fn make_response(target: Option<RouteTarget>, caller_ext: u16) -> AgiResponse {
    let outbound_trunk = target.as_ref().and_then(|t| match t {
        RouteTarget::External(_) => OUTBOUND_TRUNK_MAP.get(&caller_ext).copied(),
        _ => None,
    });
    let (status, is_internal_dest) = match target {
        Some(RouteTarget::Internal(_)) => ("SUCCESS", "TRUE"),
        Some(RouteTarget::External(_)) => ("SUCCESS", "FALSE"),
        None => ("FAILED", "FALSE"),
    };
    AgiResponse { status, is_internal_dest, target, outbound_trunk }
}

fn dispatch_route(raw_input: &str, caller_id_ext: u16, call_type: CallType) -> AgiResponse {
    let target = match call_type {
        CallType::Inbound => raw_input.parse::<u64>().ok().and_then(route_inbound),
        CallType::OldShort => raw_input.parse::<u16>().ok().and_then(route_outbound_short),
        CallType::Normalized => normalize_number(raw_input).map(route_normalized),
    };
    make_response(target, caller_id_ext)
}

fn read_agi_variables() -> HashMap<String, String> {
    let stdin = io::stdin();
    let mut variables = HashMap::new();
    let mut reader = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { break; }
                if let Some((key, value)) = trimmed.split_once(':') {
                    variables.insert(key.trim().to_lowercase(), value.trim().to_string());
                }
            }
            Err(e) => {
                eprintln!("ERROR reading AGI variable: {}", e);
                break;
            }
        }
    }
    variables
}

fn send_agi_command(command: &str) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{}", command);
    let _ = stdout.flush();
}

fn set_variable(name: &str, value: &str) {
    send_agi_command(&format!("SET VARIABLE {} \"{}\"", name, value));
}

fn verbose(message: &str, level: u8) {
    send_agi_command(&format!("VERBOSE \"{}\" {}", message, level));
}

fn main() {
    if let Err(e) = run_router() {
        verbose(&format!("CRITICAL AGI ERROR: {}", e), VERBOSE_LEVEL_WARNING);
    }
}

fn run_router() -> Result<(), Box<dyn std::error::Error>> {
    let vars = read_agi_variables();
    let raw_input = vars.get("agi_arg_1").map(|s| s.as_str()).unwrap_or("");
    let call_type_str = vars.get("agi_arg_2").map(|s| s.as_str()).unwrap_or("unknown");
    let caller_id_ext = vars.get("agi_arg_3").and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);

    let call_type = match call_type_str {
        "inbound" => CallType::Inbound,
        "old_short" => CallType::OldShort,
        "city_6" | "federal_plus" | "federal_7" | "federal_8" => CallType::Normalized,
        _ => {
            verbose(&format!("WARNING: Unknown call type '{}'", call_type_str), VERBOSE_LEVEL_WARNING);
            set_variable(ROUTE_STATUS, "FAILED");
            set_variable(TARGET_EXT, "0");
            return Ok(());
        }
    };

    let response = dispatch_route(raw_input, caller_id_ext, call_type);

    set_variable(ROUTE_STATUS, response.status);
    set_variable(IS_INTERNAL_DEST, response.is_internal_dest);

    if let Some(target) = &response.target {
        match target {
            RouteTarget::Internal(ext) => {
                set_variable(TARGET_EXT, itoa::Buffer::new().format(*ext));
                verbose(&format!("Route SUCCESS -> Internal EXT {}", ext), VERBOSE_LEVEL_SUCCESS);
            }
            RouteTarget::External(num) => {
                set_variable(OUT_NUMBER, itoa::Buffer::new().format(*num));
                if let Some(trunk) = response.outbound_trunk {
                    set_variable(DIAL_TRUNK, itoa::Buffer::new().format(trunk));
                    verbose(&format!("Route SUCCESS -> External {} via TRUNK {}", num, trunk), VERBOSE_LEVEL_SUCCESS);
                } else {
                    set_variable(ROUTE_STATUS, "FAILED");
                    verbose(&format!("Route FAILED: No DIAL_TRUNK for ext {}", caller_id_ext), VERBOSE_LEVEL_WARNING);
                }
            }
        }
    } else {
        set_variable(ROUTE_STATUS, "FAILED");
        verbose(&format!("Route FAILED: No destination for {}", raw_input), VERBOSE_LEVEL_WARNING);
    }

    Ok(())
}