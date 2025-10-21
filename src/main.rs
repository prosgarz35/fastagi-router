use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use itoa;
use once_cell::sync::Lazy;

// --- Константы нормализации ---
const FEDERAL_PREFIX: u64 = 7;
const NORMALIZED_CITY_CODE: u64 = 73843;
const CITY_MULTIPLIER: u64 = 1_000_000;
const FEDERAL_NUMBER_BASE: u64 = 10_000_000_000;

// --- Константы AGI и статусов ---
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

// --- Константы SIP-доменов ---
const EXTERNAL_DOMAIN: &str = "multifon.ru";

// --- ENUMs и Structs ---

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
    InvalidNumberFormat,
    NoRouteFound,
    NoTrunkAvailable,
    UnknownCallType,
}

#[derive(Debug)]
struct AgiResponse {
    status: &'static str,
    is_internal_dest: &'static str,
    target: RouteTarget,
    outbound_trunk: Option<u64>,
}

// --- Статические карты (MAPs) ---

static INBOUND_MAP: Lazy<HashMap<u64, u16>> = Lazy::new(|| {
    HashMap::from([
        (79235253998, 501), (73843602313, 501), (79235254061, 502), (73843601773, 502),
        (73843731773, 502), (79235254150, 503), (79235254132, 504), (73843602414, 504),
        (79235254389, 505), (79235254439, 506), (73843601771, 506), (79235254667, 507),
        (73843600912, 507), (79235254706, 508), (73843600911, 508), (73843731458, 508),
        (79235255049, 509), (73843601331, 509), (73843731313, 509), (79235255136, 510),
        (73843601221, 510), (73843731500, 510),
    ])
});

static OUTBOUND_TRUNK_MAP: Lazy<HashMap<u16, u64>> = Lazy::new(|| {
    HashMap::from([
        (501, 79235253998), (502, 79235254061), (503, 79235254150),
        (504, 79235254132), (505, 79235254389), (506, 79235254439),
        (507, 79235254667), (508, 79235254706), (509, 79235255049),
        (510, 79235255136),
    ])
});

// --- Функции нормализации и маршрутизации ---

/// Нормализует телефонный номер в федеральный формат 7XXXXXXXXXX.
/// Использует прямой парсинг цифр без аллокаций (максимальная производительность).
fn normalize_number(raw: &str) -> Option<u64> {
    let mut num: u64 = 0;
    let mut len = 0;
    
    for c in raw.chars() {
        if c.is_ascii_digit() {
            if len >= 11 { return None; }
            num = num * 10 + (c as u64 - b'0' as u64); 
            len += 1;
        }
    }
    
    if !(6..=11).contains(&len) {
        return None;
    }
    
    match len {
        11 => {
            let first_digit = num / FEDERAL_NUMBER_BASE;
            match first_digit {
                7 => Some(num),
                8 => Some(FEDERAL_PREFIX * FEDERAL_NUMBER_BASE + (num % FEDERAL_NUMBER_BASE)),
                _ => None,
            }
        },
        10 => Some(FEDERAL_PREFIX * FEDERAL_NUMBER_BASE + num),
        6 => Some(NORMALIZED_CITY_CODE * CITY_MULTIPLIER + num), 
        _ => None,
    }
}

fn route_inbound(trunk_number: u64) -> Option<RouteTarget> {
    INBOUND_MAP.get(&trunk_number).copied().map(RouteTarget::Internal)
}

fn route_normalized(num: u64) -> RouteTarget {
    INBOUND_MAP.get(&num).copied().map_or(RouteTarget::External(num), RouteTarget::Internal)
}

/// Создает AgiResponse. Проверяет наличие исходящего транка для External вызовов.
fn make_response(target: RouteTarget, caller_ext: u16) -> Result<AgiResponse, RouteError> {
    let mut outbound_trunk = None;

    if matches!(target, RouteTarget::External(_)) {
        let trunk = OUTBOUND_TRUNK_MAP.get(&caller_ext).copied()
            .ok_or(RouteError::NoTrunkAvailable)?;
        outbound_trunk = Some(trunk);
    }
    
    let (status, is_internal_dest) = match target {
        RouteTarget::Internal(_) => (STATUS_SUCCESS, INTERNAL_TRUE),
        RouteTarget::External(_) => (STATUS_SUCCESS, INTERNAL_FALSE),
    };
    
    Ok(AgiResponse { status, is_internal_dest, target, outbound_trunk })
}

/// Главный диспетчер маршрутов с явной обработкой ошибок через Result.
fn dispatch_route(raw_input: &str, caller_id_ext: u16, call_type: CallType) -> Result<AgiResponse, RouteError> {
    let target = match call_type {
        CallType::Inbound => {
            let trunk_number = raw_input.parse::<u64>()
                .map_err(|_| RouteError::InvalidNumberFormat)?;
            route_inbound(trunk_number).ok_or(RouteError::NoRouteFound)?
        },
        CallType::Normalized => {
            let normalized = normalize_number(raw_input)
                .ok_or(RouteError::InvalidNumberFormat)?;
            route_normalized(normalized)
        },
    };
    
    make_response(target, caller_id_ext)
}

// --- Функции AGI-интерфейса и IO ---

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
            Err(_) => break,
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

/// Основной цикл AGI-роутера.
fn run_router() -> Result<(), Box<dyn std::error::Error>> {
    let vars = read_agi_variables();
    let raw_input = vars.get("agi_arg_1").map(String::as_str).unwrap_or("");
    let call_type_str = vars.get("agi_arg_2").map(String::as_str).unwrap_or("unknown");
    
    let caller_id_ext = if let Some(ext_str) = vars.get("agi_arg_3") {
        ext_str.parse::<u16>().unwrap_or(0)
    } else { 0 };

    let call_type = match call_type_str {
        "inbound" => CallType::Inbound,
        "normalized" => CallType::Normalized,
        _ => {
            set_variable(ROUTE_STATUS, STATUS_FAILED);
            set_variable(TARGET_EXT, "0");
            verbose(&format!("Route FAILED: Unknown call type '{}'", call_type_str), VERBOSE_LEVEL_WARNING);
            return Ok(());
        }
    };

    match dispatch_route(raw_input, caller_id_ext, call_type) {
        Ok(response) => {
            set_variable(ROUTE_STATUS, response.status);
            set_variable(IS_INTERNAL_DEST, response.is_internal_dest);

            match response.target {
                RouteTarget::Internal(ext) => {
                    set_variable(TARGET_EXT, &ext.to_string());
                    verbose(&format!("Route SUCCESS -> Internal {}", ext), VERBOSE_LEVEL_SUCCESS);
                }
                RouteTarget::External(num) => {
                    let external_uri = format!("sip:{}@{}", num, EXTERNAL_DOMAIN);
                    set_variable(OUT_NUMBER, &external_uri);
                    
                    if let Some(trunk) = response.outbound_trunk {
                        set_variable(DIAL_TRUNK, itoa::Buffer::new().format(trunk));
                        verbose(&format!("Route SUCCESS -> External {}@{} via TRUNK {}", num, EXTERNAL_DOMAIN, trunk), VERBOSE_LEVEL_SUCCESS);
                    }
                }
            }
        }
        Err(error) => {
            set_variable(ROUTE_STATUS, STATUS_FAILED);
            let error_msg = match error {
                RouteError::InvalidNumberFormat => format!("Invalid number format: {}", raw_input),
                RouteError::NoRouteFound => format!("No route found for: {}", raw_input),
                RouteError::NoTrunkAvailable => format!("No trunk available for extension: {}", caller_id_ext),
                RouteError::UnknownCallType => "Unknown call type error propagation".to_string(), 
            };
            verbose(&format!("Route FAILED: {}", error_msg), VERBOSE_LEVEL_WARNING);
        }
    }

    Ok(())
}