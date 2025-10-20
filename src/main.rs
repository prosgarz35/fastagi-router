use asterisk_agi::agi::{Agi, AgiError}; // ИСПРАВЛЕНО
use std::collections::HashMap;
use itoa;
use once_cell::sync::Lazy;

const CITY_PREFIX_U64: u64 = 73843;
const TEN_BILLION: u64 = 10_000_000_000;

const ROUTE_STATUS: &str = "ROUTE_STATUS";
const IS_INTERNAL_DEST: &str = "IS_INTERNAL_DEST";
const TARGET_EXT: &str = "TARGET_EXT";
const OUT_NUMBER: &str = "OUT_NUMBER";
const DIAL_TRUNK: &str = "DIAL_TRUNK";

#[derive(Debug)]
enum RouteTarget {
    Internal(u16),
    External(u64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum CallType {
    Inbound,
    OldShort,
    City6,
    FederalPlus,
    Federal7,
    Federal8,
    Unknown,
}

struct AgiResponse {
    status: &'static str,
    is_internal_dest: &'static str,
    target: Option<RouteTarget>,
    outbound_trunk: Option<u64>,
}

static INBOUND_MAP: Lazy<HashMap<u64, u16>> = Lazy::new(|| {
    HashMap::from([
        (79235253998, 501), (73843602313, 501),
        (79235254061, 502), (73843601773, 502), (73843731773, 502),
        (79235254150, 503),
        (79235254132, 504), (73843602414, 504),
        (79235254389, 505),
        (79235254439, 506), (73843601771, 506),
        (79235254667, 507), (73843600912, 507),
        (79235254706, 508), (73843600911, 508), (73843731458, 508),
        (79235255049, 509), (73843601331, 509), (73843731313, 509),
        (79235255136, 510), (73843601221, 510), (73843731500, 510),
    ])
});

static OUTBOUND_TRUNK_MAP: Lazy<HashMap<u16, u64>> = Lazy::new(|| {
    HashMap::from([
        (501, 79235253998),
        (502, 79235254061),
        (503, 79235254150),
        (504, 79235254132),
        (505, 79235254389),
        (506, 79235254439),
        (507, 79235254667),
        (508, 79235254706),
        (509, 79235255049),
        (510, 79235255136),
    ])
});

static SHORT_CODE_MAP: Lazy<HashMap<u16, u16>> = Lazy::new(|| {
    HashMap::from([
        (104, 501),
        (135, 502),
        (119, 502),
        (111, 508),
        (106, 509),
    ])
});

fn parse_number_with_cleaning(raw_input: &str) -> Option<u64> {
    raw_input.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
}

fn normalize_to_7(num_u64: u64) -> u64 {
    7 * TEN_BILLION + (num_u64 % TEN_BILLION)
}

fn route_inbound(trunk_number: u64) -> Option<RouteTarget> {
    INBOUND_MAP.get(&trunk_number).copied().map(RouteTarget::Internal)
}

fn route_outbound_short(short_code: u16) -> Option<RouteTarget> {
    SHORT_CODE_MAP.get(&short_code).copied().map(RouteTarget::Internal)
}

fn route_by_external_number(number: u64) -> Option<RouteTarget> {
    INBOUND_MAP.get(&number).copied().map(RouteTarget::Internal).or(Some(RouteTarget::External(number)))
}

fn make_response(target: Option<RouteTarget>, outbound_trunk: Option<u64>) -> AgiResponse {
    match target {
        Some(RouteTarget::Internal(_)) => AgiResponse { status: "SUCCESS", is_internal_dest: "TRUE", target, outbound_trunk: None },
        Some(RouteTarget::External(_)) => AgiResponse { status: "SUCCESS", is_internal_dest: "FALSE", target, outbound_trunk },
        None => AgiResponse { status: "FAILED", is_internal_dest: "FALSE", target: None, outbound_trunk: None },
    }
}

fn dispatch_route(raw_input: &str, caller_id_ext: u16, call_type: CallType) -> AgiResponse {
    let target = match call_type {
        CallType::Inbound => parse_number_with_cleaning(raw_input).and_then(route_inbound),
        CallType::OldShort => raw_input.parse::<u16>().ok().and_then(route_outbound_short),
        CallType::City6 => raw_input.parse::<u64>().ok().map(|n| CITY_PREFIX_U64 * 1_000_000 + n).and_then(route_by_external_number),
        CallType::FederalPlus => parse_number_with_cleaning(raw_input).map(normalize_to_7).and_then(route_by_external_number),
        CallType::Federal7 => raw_input.parse::<u64>().ok().and_then(route_by_external_number),
        CallType::Federal8 => raw_input.parse::<u64>().ok().map(normalize_to_7).and_then(route_by_external_number),
        CallType::Unknown => None,
    };

    let outbound_trunk = match &target {
        Some(RouteTarget::External(_)) => OUTBOUND_TRUNK_MAP.get(&caller_id_ext).copied(),
        _ => None,
    };

    make_response(target, outbound_trunk)
}

fn send_agi_response(agi: &mut Agi, response: &AgiResponse) -> Result<(), AgiError> {
    let mut buffer = itoa::Buffer::new();
    agi.set_variable(ROUTE_STATUS, response.status)?;
    agi.set_variable(IS_INTERNAL_DEST, response.is_internal_dest)?;
    if let Some(target) = &response.target {
        match target {
            RouteTarget::Internal(ext) => { agi.set_variable(TARGET_EXT, buffer.format(*ext))?; }
            RouteTarget::External(num) => {
                agi.set_variable(OUT_NUMBER, buffer.format(*num))?;
                if let Some(trunk) = response.outbound_trunk {
                    agi.set_variable(DIAL_TRUNK, buffer.format(trunk))?;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), AgiError> {
    let mut agi = Agi::new()?;
    let args = agi.read_args()?; 
    let raw_input = args.get(0).map(|s| s.as_str()).unwrap_or_default();
    let call_type_str = args.get(1).map(|s| s.as_str()).unwrap_or("unknown");
    let caller_id_ext = args.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);

    let call_type = match call_type_str {
        "inbound" => CallType::Inbound,
        "old_short" => CallType::OldShort,
        "city_6" => CallType::City6,
        "federal_plus" => CallType::FederalPlus,
        "federal_7" => CallType::Federal7,
        "federal_8" => CallType::Federal8,
        _ => CallType::Unknown,
    };

    let response = dispatch_route(raw_input, caller_id_ext, call_type);
    send_agi_response(&mut agi, &response)?;
    Ok(())
}