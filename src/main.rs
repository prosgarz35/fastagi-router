use std::{
    borrow::Cow,
    io::{self, BufRead, Write, stdout},
};
use phf::phf_map;

const SIX_DIGIT_PREFIX: &str = "73843";

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

fn set_var<W: Write>(w: &mut W, name: &str, value: &str) -> io::Result<()> {
    writeln!(w, "SET VARIABLE {} \"{}\"", name, value)?;
    w.flush()
}

enum LookupStatus<'a> {
    Internal(&'a str),
    External(String), 
    Failure(&'a str),
}

impl<'a> LookupStatus<'a> {
    fn into_parts(self) -> (&'static str, &'static str, Cow<'a, str>, &'a str) {
        match self {
            Self::Internal(t) => ("TRUE", "TRUE", Cow::Borrowed(t), ""),
            Self::External(t) => ("TRUE", "FALSE", Cow::Owned(t), ""),
            Self::Failure(r) => ("FALSE", "FALSE", Cow::Borrowed(""), r),
        }
    }
}

fn set_lookup<W: Write>(status: LookupStatus, w: &mut W) -> io::Result<()> {
    let (succ, internal, target_cow, reason) = status.into_parts();
    let target = target_cow.as_ref();
    set_var(w, "LOOKUP_SUCCESS", succ)?;
    set_var(w, "IS_INTERNAL_DEST", internal)?;
    set_var(w, "DIAL_TARGET", target)?; 
    if succ == "FALSE" { set_var(w, "LOOKUP_REASON", reason)?; }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode { Inbound, Outbound }

impl Mode {
    fn from_str(s: &str) -> Self {
        match s { "inbound" => Self::Inbound, _ => Self::Outbound }
    }
}

struct AgiVars { dialed: String, caller: String, mode: Mode }

impl AgiVars {
    fn from_stdin() -> io::Result<Self> {
        let mut dialed = String::new();
        let mut caller = String::new();
        let mut mode = Mode::Outbound;
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            let l = line.trim();
            if l.is_empty() { break; }
            if let Some((k, v)) = l.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                match k {
                    "agi_arg_1" => dialed = v.to_owned(),
                    "agi_arg_2" => caller = v.to_owned(),
                    "agi_arg_3" => mode = Mode::from_str(v),
                    _ => {}
                }
            }
        }
        Ok(Self { dialed, caller, mode })
    }
}

fn just_sanitize(s: &str) -> Option<Cow<'_, str>> {
    let digits = if s.chars().all(|c| c.is_ascii_digit()) { 
        Cow::Borrowed(s)
    } else {
        Cow::Owned(s.chars().filter(|c| c.is_ascii_digit()).collect())
    };
    (!digits.is_empty()).then_some(digits)
}

fn sanitize_and_normalize(s: &str) -> Option<Cow<'_, str>> {
    let digits = just_sanitize(s)?;
    match digits.len() {
        3 => Some(digits),
        6 => {
            let mut n = String::with_capacity(SIX_DIGIT_PREFIX.len() + 6);
            n.push_str(SIX_DIGIT_PREFIX);
            n.push_str(&digits);
            Some(Cow::Owned(n))
        }
        11 => {
            let first = digits.as_bytes()[0];
            if first == b'7' { Some(digits) }
            else if first == b'8' {
                let mut n = String::with_capacity(11);
                n.push('7');
                n.push_str(&digits[1..]);
                Some(Cow::Owned(n))
            } else { None }
        }
        _ => None,
    }
}

fn handle_outbound(vars: AgiVars, w: &mut impl Write) -> io::Result<LookupStatus<'static>> {
    if let Some(caller) = just_sanitize(&vars.caller) {
        if caller.len() == 3 {
            if let Some(&trunk) = EXT_TO_TRUNK.get(&caller) {
                set_var(w, "DIAL_TRUNK", trunk)?;
            }
        }
    }
    let normalized = match sanitize_and_normalize(&vars.dialed).ok_or(
        LookupStatus::Failure("normalize_failed_wrong_length"),
    ) {
        Ok(n) => n,
        Err(status) => return Ok(status),
    };
    Ok(match NUMBER_TO_EXT.get(&normalized) {
        Some(&ext) => LookupStatus::Internal(ext),
        None => if normalized.len() == 3 {
            LookupStatus::Failure("short_internal_rejected")
        } else {
            LookupStatus::External(match normalized {
                Cow::Borrowed(s) => s.to_owned(),
                Cow::Owned(s) => s,
            })
        },
    })
}

fn run_lookup(vars: AgiVars, w: &mut impl Write) -> io::Result<()> {
    let status = match vars.mode {
        Mode::Outbound => handle_outbound(vars, w)?,
        Mode::Inbound => {
            let dialed = match just_sanitize(&vars.dialed) {
                None => {
                    set_lookup(LookupStatus::Failure("empty_dial"), w)?;
                    return Ok(());
                }
                Some(d) => d,
            };
            match NUMBER_TO_EXT.get(&dialed) {
                Some(&ext) => LookupStatus::Internal(ext),
                None => LookupStatus::Failure("unknown_inbound_did"),
            }
        }
    };
    set_lookup(status, w)
}

fn main() -> io::Result<()> {
    let mut out = stdout().lock();
    let vars = AgiVars::from_stdin()?;
    run_lookup(vars, &mut out)
}