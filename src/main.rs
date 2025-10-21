use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use itoa;
use once_cell::sync::Lazy;

const NORMALIZED_CITY_CODE: u64 = 73843;
const CITY_MULTIPLIER: u64 = 1_000_000;

const ROUTE_STATUS: &str = "ROUTE_STATUS";
const IS_INTERNAL_DEST: &str = "IS_INTERNAL_DEST";
const TARGET_EXT: &str = "TARGET_EXT";
const OUT_NUMBER: &str = "OUT_NUMBER";
const DIAL_TRUNK: &str = "DIAL_TRUNK";
const VERBOSE_LEVEL_WARNING: u8 = 1;
const VERBOSE_LEVEL_SUCCESS: u8 = 3;
const STATUS_SUCCESS: &str = "SUCCESS";
const STATUS_FAILED: &str = "FAILED";
const INTERNAL_TRUE: &str = "TRUE";
const INTERNAL_FALSE: &str = "FALSE";

#[derive(Debug)]
enum RouteTarget {
    Internal(u16),
    External(u64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum CallType {
    Inbound,
    Normalized,
}

#[derive(Debug)]
enum RouteError {
    NoRouteFound,
    NoTrunkAvailable,
    InvalidFormat,
    ShortNumberNotMapped,
}

#[derive(Debug)]
struct AgiResponse {
    status: &'static str,
    is_internal_dest: &'static str,
    target: RouteTarget,
    outbound_trunk: Option<u64>,
}

static NUMBER_MAPPING: Lazy<HashMap<u64, u16>> = Lazy::new(|| {
    HashMap::from([
        (79235253998, 501), (79235254061, 502), (79235254150, 503), (79235254132, 504), (79235254389, 505),
        (79235254439, 506), (79235254667, 507), (79235254706, 508), (79235255049, 509), (79235255136, 510),
        (73843602313, 501), (73843601773, 502), (73843731773, 502), (73843602414, 504), (73843601771, 506),
        (73843600912, 507), (73843600911, 508), (73843731458, 508), (73843601331, 509), (73843731313, 509),
        (73843601221, 510), (73843731500, 510), (104, 501), (135, 502), (119, 502),
        (111, 508), (106, 509), (108, 510),
    ])
});

static EXTENSION_TRUNK_MAP: Lazy<HashMap<u16, u64>> = Lazy::new(|| {
    HashMap::from([
        (501, 79235253998), (502, 79235254061), (503, 79235254150), (504, 79235254132), (505, 79235254389),
        (506, 79235254439), (507, 79235254667), (508, 79235254706), (509, 79235255049), (510, 79235255136),
    ])
});

fn normalize_number(raw: &str) -> Result<u64, RouteError> {
    let clean = raw.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    
    match clean.len() {
        3 => {
            clean.parse().map_err(|_| RouteError::InvalidFormat)
        },
        6 => {
            let num: u64 = clean.parse().map_err(|_| RouteError::InvalidFormat)?;
            Ok(NORMALIZED_CITY_CODE * CITY_MULTIPLIER + num)
        },
        11 => {
            let num: u64 = clean.parse().map_err(|_| RouteError::InvalidFormat)?;
            let first_digit = num / 10_000_000_000;
            
            match first_digit {
                7 => Ok(num),
                8 => Ok(7_000_000_0000 + (num % 10_000_000_000)),
                _ => Err(RouteError::InvalidFormat),
            }
        },
        _ => Err(RouteError::InvalidFormat),
    }
}

fn route_inbound(trunk_number: u64) -> Option<RouteTarget> {
    NUMBER_MAPPING.get(&trunk_number).copied().map(RouteTarget::Internal)
}

fn route_normalized(num: u64, _caller_ext: u16) -> Result<RouteTarget, RouteError> {
    if let Some(&ext) = NUMBER_MAPPING.get(&num) {
        Ok(RouteTarget::Internal(ext))
    } else {
        if num < 1000 {
            Err(RouteError::ShortNumberNotMapped)
        } else {
            Ok(RouteTarget::External(num))
        }
    }
}

fn make_response(target: RouteTarget, caller_ext: u16) -> Result<AgiResponse, RouteError> {
    let outbound_trunk = match target {
        RouteTarget::External(_) => {
            let trunk = EXTENSION_TRUNK_MAP.get(&caller_ext)
                .ok_or(RouteError::NoTrunkAvailable)?;
            Some(*trunk)
        }
        RouteTarget::Internal(_) => None,
    };
    
    let (status, is_internal_dest) = match target {
        RouteTarget::Internal(_) => (STATUS_SUCCESS, INTERNAL_TRUE),
        RouteTarget::External(_) => (STATUS_SUCCESS, INTERNAL_FALSE),
    };
    
    Ok(AgiResponse { status, is_internal_dest, target, outbound_trunk })
}

fn dispatch_route(raw_input: &str, caller_id_ext: u16, call_type: CallType) -> Result<AgiResponse, RouteError> {
    let target = match call_type {
        CallType::Inbound => {
            let trunk_number: u64 = raw_input.parse().map_err(|_| RouteError::InvalidFormat)?;
            route_inbound(trunk_number).ok_or(RouteError::NoRouteFound)?
        },
        CallType::Normalized => {
            let normalized = normalize_number(raw_input)?;
            route_normalized(normalized, caller_id_ext)?
        },
    };
    
    make_response(target, caller_id_ext)
}

fn read_agi_variables() -> HashMap<String, String> { 
    let stdin = io::stdin();
    let mut variables = HashMap::new();
    let mut reader = stdin.lock();
    let mut line = String::new();
    
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() { break; }
        if let Some((key, value)) = trimmed.split_once(':') {
            variables.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
        line.clear();
    }
    variables
}

fn send_agi_command(command: &str) -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", command)?;
    stdout.flush()?;
    Ok(())
}

fn set_variable(name: &str, value: &str) {
    if let Err(e) = send_agi_command(&format!("SET VARIABLE {} \"{}\"", name, value)) {
        eprintln!("Failed to set variable {}: {}", name, e);
    }
}

fn verbose(message: &str, level: u8) {
    if let Err(e) = send_agi_command(&format!("VERBOSE \"{}\" {}", message, level)) {
        eprintln!("Failed to send verbose: {}", e);
    }
}

fn main() {
    if let Err(e) = run_router() {
        verbose(&format!("AGI ERROR: {}", e), VERBOSE_LEVEL_WARNING);
    }
}

fn run_router() -> Result<(), Box<dyn std::error::Error>> {
    let vars = read_agi_variables();
    let raw_input = vars.get("agi_arg_1").map(String::as_str).unwrap_or("");
    let call_type_str = vars.get("agi_arg_2").map(String::as_str).unwrap_or("");
    let caller_id_str = vars.get("agi_arg_3").map(String::as_str).unwrap_or("");
    
    let caller_id_ext = caller_id_str.parse::<u16>().unwrap_or(0);
    let mut buf = itoa::Buffer::new();

    let call_type = match call_type_str {
        "inbound" => CallType::Inbound,
        "normalized" => CallType::Normalized,
        _ => {
            set_variable(ROUTE_STATUS, STATUS_FAILED);
            verbose(&format!("Unknown call type: {}", call_type_str), VERBOSE_LEVEL_WARNING);
            return Ok(());
        }
    };

    match dispatch_route(raw_input, caller_id_ext, call_type) {
        Ok(response) => {
            set_variable(ROUTE_STATUS, response.status);
            set_variable(IS_INTERNAL_DEST, response.is_internal_dest);

            match response.target {
                RouteTarget::Internal(ext) => {
                    set_variable(TARGET_EXT, buf.format(ext));
                    set_variable(OUT_NUMBER, "");
                    set_variable(DIAL_TRUNK, "");
                    
                    match call_type {
                        CallType::Inbound => {
                            verbose(&format!("[INBOUND] Trunk: {} -> Internal: {}", raw_input, ext), VERBOSE_LEVEL_SUCCESS);
                        },
                        CallType::Normalized => {
                            verbose(&format!("[OUTBOUND] Caller: {} -> Internal: {} (Input: {})", caller_id_ext, ext, raw_input), VERBOSE_LEVEL_SUCCESS);
                        },
                    }
                }
                RouteTarget::External(num) => {
                    set_variable(OUT_NUMBER, buf.format(num));
                    if let Some(trunk) = response.outbound_trunk {
                        set_variable(DIAL_TRUNK, buf.format(trunk));
                        verbose(&format!("[OUTBOUND] Caller: {} -> External: {} via Trunk: {} (Input: {})", 
                                         caller_id_ext, num, trunk, raw_input), VERBOSE_LEVEL_SUCCESS);
                    } else {
                        verbose(&format!("[OUTBOUND] Caller: {} -> External: {} (no trunk specified)", 
                                         caller_id_ext, num), VERBOSE_LEVEL_WARNING);
                    }
                }
            }
        }
        Err(error) => {
            set_variable(ROUTE_STATUS, STATUS_FAILED);
            let error_msg = match error {
                RouteError::NoRouteFound => format!("No route found for: {}", raw_input),
                RouteError::InvalidFormat => format!("Invalid number format: {}", raw_input),
                RouteError::NoTrunkAvailable => {
                    if caller_id_ext == 0 {
                        format!("No trunk available (invalid caller ID: {})", caller_id_str)
                    } else {
                        format!("No trunk available for extension: {}", caller_id_ext)
                    }
                },
                RouteError::ShortNumberNotMapped => format!("Short number not mapped to internal extension: {}", raw_input),
            };
            verbose(&format!("[ROUTE-ERROR] Caller: {} -> {}: {}", caller_id_ext, error_msg, raw_input), VERBOSE_LEVEL_WARNING);
        }
    }

    Ok(())
}