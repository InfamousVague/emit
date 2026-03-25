use async_trait::async_trait;
use crate::launcher::CommandEntry;
use super::CommandProvider;
use chrono::{Local, NaiveTime, Timelike};
use chrono_tz::Tz;
use regex::Regex;
use std::sync::LazyLock;

pub struct CalculatorProvider;

impl CalculatorProvider {
    pub fn new() -> Self { Self }
}

// ---------------------------------------------------------------------------
// Regex patterns (compiled once)
// ---------------------------------------------------------------------------

// Strict unit pattern: "5 kg to lbs"
static UNIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(-?[\d,._]+)\s*([a-zµ°³²/]+(?:\s+oz)?)\s+(?:to|in|as|->|=>)\s+([a-zµ°³²/]+(?:\s+oz)?)$").unwrap()
});

// Natural language unit pattern: "how many cups in a pint", "convert 5 kg to lbs"
static UNIT_NATURAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:how\s+many|convert|what(?:'s|\s+is)?)\s+(?:(\d[\d,._]*)\s+)?([a-z/°µ]+(?:\s+oz)?)\s+(?:to|in|into|are\s+in(?:\s+a)?|=)\s+(?:a\s+)?(?:(\d[\d,._]*)\s+)?([a-z/°µ]+(?:\s+oz)?)").unwrap()
});

// Strict time pattern: "10pm est in dubai"
static TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s+([a-z/_\s]+?)\s+(?:to|in)\s+([a-z/_\s]+?)$").unwrap()
});

// Natural language time: "what time is 5am est in dubai", "5am my time to dubai"
static TIME_NATURAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:what\s+(?:time\s+is|is)\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s+(?:in\s+)?([a-z/_\s]+?)\s+(?:to|in)\s+([a-z/_\s]+?)$").unwrap()
});

// ---------------------------------------------------------------------------
// Input normalization — handle typos and filler
// ---------------------------------------------------------------------------

fn normalize_query(raw: &str) -> String {
    let s = raw.trim().to_lowercase();
    // Strip common filler words/prefixes
    let s = strip_prefix_words(&s, &[
        "what is", "what's", "whats", "how much is", "how many",
        "waht time is", "what time is", "wht time is", "wat time is",
        "convert", "calculate", "calc", "tell me",
    ]);
    s.trim().to_string()
}

fn strip_prefix_words<'a>(s: &'a str, prefixes: &[&str]) -> &'a str {
    for prefix in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            return rest.trim();
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Unit conversion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum UnitKind { Length, Weight, Volume, Temperature, Digital, Time, Speed }

#[derive(Debug, Clone, Copy)]
struct UnitDef {
    kind: UnitKind,
    to_base: f64,
    label: &'static str,
}

fn lookup_unit(raw: &str) -> Option<UnitDef> {
    let s = raw.trim().to_lowercase().replace(' ', "");
    let (kind, to_base, label) = match s.as_str() {
        // Length (base: meters)
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => (UnitKind::Length, 0.001, "mm"),
        "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => (UnitKind::Length, 0.01, "cm"),
        "m" | "meter" | "meters" | "metre" | "metres" => (UnitKind::Length, 1.0, "m"),
        "km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => (UnitKind::Length, 1000.0, "km"),
        "in" | "inch" | "inches" | "\"" => (UnitKind::Length, 0.0254, "in"),
        "ft" | "foot" | "feet" => (UnitKind::Length, 0.3048, "ft"),
        "yd" | "yard" | "yards" => (UnitKind::Length, 0.9144, "yd"),
        "mi" | "mile" | "miles" => (UnitKind::Length, 1609.344, "mi"),
        "nm" | "nmi" | "nauticalmile" | "nauticalmiles" => (UnitKind::Length, 1852.0, "nmi"),

        // Weight (base: grams)
        "mg" | "milligram" | "milligrams" => (UnitKind::Weight, 0.001, "mg"),
        "g" | "gram" | "grams" => (UnitKind::Weight, 1.0, "g"),
        "kg" | "kilogram" | "kilograms" | "kilo" | "kilos" => (UnitKind::Weight, 1000.0, "kg"),
        "oz" | "ounce" | "ounces" => (UnitKind::Weight, 28.3495, "oz"),
        "lb" | "lbs" | "pound" | "pounds" => (UnitKind::Weight, 453.592, "lbs"),
        "st" | "stone" | "stones" => (UnitKind::Weight, 6350.29, "st"),
        "ton" | "tons" | "tonne" | "tonnes" | "mt" => (UnitKind::Weight, 1_000_000.0, "t"),

        // Volume (base: milliliters)
        "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => (UnitKind::Volume, 1.0, "ml"),
        "l" | "liter" | "liters" | "litre" | "litres" => (UnitKind::Volume, 1000.0, "L"),
        "gal" | "gallon" | "gallons" => (UnitKind::Volume, 3785.41, "gal"),
        "qt" | "quart" | "quarts" => (UnitKind::Volume, 946.353, "qt"),
        "pt" | "pint" | "pints" => (UnitKind::Volume, 473.176, "pt"),
        "cup" | "cups" => (UnitKind::Volume, 236.588, "cups"),
        "floz" | "fl.oz" | "fluidounce" | "fluidounces" => (UnitKind::Volume, 29.5735, "fl oz"),
        "tbsp" | "tablespoon" | "tablespoons" => (UnitKind::Volume, 14.7868, "tbsp"),
        "tsp" | "teaspoon" | "teaspoons" => (UnitKind::Volume, 4.92892, "tsp"),

        // Temperature (special handling — to_base encodes type)
        "c" | "°c" | "celsius" | "centigrade" => (UnitKind::Temperature, 0.0, "°C"),
        "f" | "°f" | "fahrenheit" => (UnitKind::Temperature, 1.0, "°F"),
        "k" | "kelvin" => (UnitKind::Temperature, 2.0, "K"),

        // Digital storage (base: bytes)
        "b" | "byte" | "bytes" => (UnitKind::Digital, 1.0, "B"),
        "kb" | "kilobyte" | "kilobytes" => (UnitKind::Digital, 1000.0, "KB"),
        "mb" | "megabyte" | "megabytes" => (UnitKind::Digital, 1e6, "MB"),
        "gb" | "gigabyte" | "gigabytes" => (UnitKind::Digital, 1e9, "GB"),
        "tb" | "terabyte" | "terabytes" => (UnitKind::Digital, 1e12, "TB"),
        "pb" | "petabyte" | "petabytes" => (UnitKind::Digital, 1e15, "PB"),
        "kib" | "kibibyte" | "kibibytes" => (UnitKind::Digital, 1024.0, "KiB"),
        "mib" | "mebibyte" | "mebibytes" => (UnitKind::Digital, 1_048_576.0, "MiB"),
        "gib" | "gibibyte" | "gibibytes" => (UnitKind::Digital, 1_073_741_824.0, "GiB"),
        "tib" | "tebibyte" | "tebibytes" => (UnitKind::Digital, 1_099_511_627_776.0, "TiB"),

        // Time (base: seconds)
        "ms" | "millisecond" | "milliseconds" => (UnitKind::Time, 0.001, "ms"),
        "s" | "sec" | "secs" | "second" | "seconds" => (UnitKind::Time, 1.0, "s"),
        "min" | "mins" | "minute" | "minutes" => (UnitKind::Time, 60.0, "min"),
        "hr" | "hrs" | "hour" | "hours" => (UnitKind::Time, 3600.0, "hr"),
        "day" | "days" => (UnitKind::Time, 86400.0, "days"),
        "week" | "weeks" | "wk" | "wks" => (UnitKind::Time, 604800.0, "weeks"),

        // Speed (base: m/s)
        "mph" => (UnitKind::Speed, 0.44704, "mph"),
        "kph" | "kmh" | "km/h" => (UnitKind::Speed, 0.277778, "km/h"),
        "m/s" | "mps" => (UnitKind::Speed, 1.0, "m/s"),
        "knot" | "knots" | "kn" | "kt" => (UnitKind::Speed, 0.514444, "knots"),
        "fps" | "ft/s" => (UnitKind::Speed, 0.3048, "ft/s"),

        _ => return None,
    };
    Some(UnitDef { kind, to_base, label })
}

fn convert_units(value: f64, from: &UnitDef, to: &UnitDef) -> Option<f64> {
    if from.kind != to.kind { return None; }

    if from.kind == UnitKind::Temperature {
        let celsius = match from.to_base as u8 {
            0 => value,
            1 => (value - 32.0) * 5.0 / 9.0,
            _ => value - 273.15,
        };
        let result = match to.to_base as u8 {
            0 => celsius,
            1 => celsius * 9.0 / 5.0 + 32.0,
            _ => celsius + 273.15,
        };
        Some(result)
    } else {
        let base = value * from.to_base;
        Some(base / to.to_base)
    }
}

fn try_unit_conversion(query: &str) -> Option<(String, String)> {
    // Try strict pattern first: "5 kg to lbs"
    if let Some(caps) = UNIT_RE.captures(query) {
        let num_str = caps[1].replace([',', '_'], "");
        let value: f64 = num_str.parse().ok()?;
        let from = lookup_unit(&caps[2])?;
        let to = lookup_unit(&caps[3])?;
        let result = convert_units(value, &from, &to)?;
        let formatted = format_number(result);
        let display = format!("{} {} = {} {}", format_number(value), from.label, formatted, to.label);
        return Some((display, formatted));
    }

    // Try natural language: "how many cups in a pint", "convert 5 kg to lbs"
    if let Some(caps) = UNIT_NATURAL_RE.captures(query) {
        // Group 1 = optional number before first unit, Group 3 = optional number before second unit
        let (value, from_raw, to_raw) = if caps.get(1).is_some() {
            // "convert 5 kg to lbs" — number is before first unit
            let v: f64 = caps[1].replace([',', '_'], "").parse().ok()?;
            (v, caps[2].to_string(), caps[4].to_string())
        } else if caps.get(3).is_some() {
            // "how many cups in 2 pints" — number before second unit, swap direction
            let v: f64 = caps[3].replace([',', '_'], "").parse().ok()?;
            (v, caps[4].to_string(), caps[2].to_string())
        } else {
            // "how many cups in a pint" — default to 1
            (1.0, caps[4].to_string(), caps[2].to_string())
        };

        let from = lookup_unit(&from_raw)?;
        let to = lookup_unit(&to_raw)?;
        let result = convert_units(value, &from, &to)?;
        let formatted = format_number(result);
        let display = format!("{} {} = {} {}", format_number(value), from.label, formatted, to.label);
        return Some((display, formatted));
    }

    None
}

// ---------------------------------------------------------------------------
// Timezone conversion
// ---------------------------------------------------------------------------

fn lookup_timezone(raw: &str) -> Option<Tz> {
    let s = raw.trim().to_lowercase().replace([' ', '-'], "");

    // Handle "my time" / "local" / "here" — use system timezone
    if matches!(s.as_str(), "mytime" | "local" | "here" | "me" | "mytz" | "localtime") {
        return local_timezone();
    }

    match s.as_str() {
        // US
        "est" | "edt" | "eastern" | "newyork" | "nyc" | "ny" => Some(chrono_tz::America::New_York),
        "cst" | "central" | "chicago" => Some(chrono_tz::America::Chicago),
        "mst" | "mdt" | "mountain" | "denver" => Some(chrono_tz::America::Denver),
        "pst" | "pdt" | "pacific" | "losangeles" | "la" | "sanfrancisco" | "sf" | "seattle" => Some(chrono_tz::America::Los_Angeles),
        "hst" | "hawaii" | "honolulu" => Some(chrono_tz::Pacific::Honolulu),
        "akst" | "akdt" | "alaska" | "anchorage" => Some(chrono_tz::America::Anchorage),

        // Americas
        "toronto" | "montreal" | "ottawa" => Some(chrono_tz::America::Toronto),
        "vancouver" | "calgary" => Some(chrono_tz::America::Vancouver),
        "mexico" | "mexicocity" => Some(chrono_tz::America::Mexico_City),
        "bogota" | "colombia" => Some(chrono_tz::America::Bogota),
        "lima" | "peru" => Some(chrono_tz::America::Lima),
        "santiago" | "chile" => Some(chrono_tz::America::Santiago),
        "buenosaires" | "argentina" => Some(chrono_tz::America::Argentina::Buenos_Aires),
        "saopaulo" | "brazil" | "brt" | "rio" => Some(chrono_tz::America::Sao_Paulo),

        // Europe
        "gmt" | "utc" | "greenwich" | "zulu" => Some(chrono_tz::UTC),
        "uk" | "london" | "england" | "britain" => Some(chrono_tz::Europe::London),
        "cet" | "paris" | "france" | "berlin" | "germany" | "amsterdam" | "netherlands" |
        "rome" | "italy" | "madrid" | "spain" | "brussels" | "belgium" |
        "vienna" | "austria" | "zurich" | "switzerland" | "frankfurt" | "munich" | "milan" => Some(chrono_tz::Europe::Paris),
        "eet" | "athens" | "greece" | "bucharest" | "romania" | "helsinki" | "finland" | "sofia" | "bulgaria" => Some(chrono_tz::Europe::Athens),
        "msk" | "moscow" | "russia" | "stpetersburg" => Some(chrono_tz::Europe::Moscow),
        "dublin" | "ireland" => Some(chrono_tz::Europe::Dublin),
        "lisbon" | "portugal" => Some(chrono_tz::Europe::Lisbon),
        "stockholm" | "sweden" | "oslo" | "norway" | "copenhagen" | "denmark" |
        "warsaw" | "poland" | "prague" | "czechia" => Some(chrono_tz::Europe::Stockholm),
        "istanbul" | "turkey" | "trt" => Some(chrono_tz::Europe::Istanbul),
        "kyiv" | "kiev" | "ukraine" => Some(chrono_tz::Europe::Kyiv),

        // Middle East
        "dubai" | "abudhabi" | "uae" | "gst" | "emirates" => Some(chrono_tz::Asia::Dubai),
        "riyadh" | "saudiarabia" | "saudi" => Some(chrono_tz::Asia::Riyadh),
        "tehran" | "iran" | "irst" => Some(chrono_tz::Asia::Tehran),
        "doha" | "qatar" => Some(chrono_tz::Asia::Qatar),
        "kuwait" => Some(chrono_tz::Asia::Kuwait),
        "bahrain" => Some(chrono_tz::Asia::Bahrain),
        "israel" | "telaviv" | "jerusalem" => Some(chrono_tz::Asia::Jerusalem),

        // South Asia
        "ist" | "india" | "mumbai" | "delhi" | "bangalore" | "bengaluru" |
        "kolkata" | "chennai" | "newdelhi" | "hyderabad" | "pune" => Some(chrono_tz::Asia::Kolkata),
        "karachi" | "pakistan" | "pkt" | "lahore" | "islamabad" => Some(chrono_tz::Asia::Karachi),
        "dhaka" | "bangladesh" => Some(chrono_tz::Asia::Dhaka),
        "colombo" | "srilanka" => Some(chrono_tz::Asia::Colombo),
        "kathmandu" | "nepal" | "npt" => Some(chrono_tz::Asia::Kathmandu),

        // East Asia
        "jst" | "tokyo" | "japan" | "osaka" => Some(chrono_tz::Asia::Tokyo),
        "kst" | "seoul" | "korea" | "southkorea" | "busan" => Some(chrono_tz::Asia::Seoul),
        "beijing" | "shanghai" | "china" | "shenzhen" | "guangzhou" |
        "hongkong" | "hk" | "hkt" => Some(chrono_tz::Asia::Shanghai),
        "taipei" | "taiwan" => Some(chrono_tz::Asia::Taipei),
        "singapore" | "sg" | "sgt" => Some(chrono_tz::Asia::Singapore),
        "bangkok" | "thailand" | "ict" => Some(chrono_tz::Asia::Bangkok),
        "jakarta" | "indonesia" | "wib" => Some(chrono_tz::Asia::Jakarta),
        "manila" | "philippines" | "pht" => Some(chrono_tz::Asia::Manila),
        "kualalumpur" | "malaysia" | "myt" | "kl" => Some(chrono_tz::Asia::Kuala_Lumpur),
        "hanoi" | "vietnam" | "hochiminhcity" | "hochiminh" | "saigon" => Some(chrono_tz::Asia::Ho_Chi_Minh),

        // Oceania
        "aest" | "sydney" | "melbourne" | "australia" | "brisbane" => Some(chrono_tz::Australia::Sydney),
        "perth" | "awst" => Some(chrono_tz::Australia::Perth),
        "adelaide" | "acst" => Some(chrono_tz::Australia::Adelaide),
        "auckland" | "newzealand" | "nz" | "nzst" | "wellington" => Some(chrono_tz::Pacific::Auckland),

        // Africa
        "cairo" | "egypt" => Some(chrono_tz::Africa::Cairo),
        "lagos" | "nigeria" | "wat" => Some(chrono_tz::Africa::Lagos),
        "johannesburg" | "southafrica" | "sast" | "capetown" => Some(chrono_tz::Africa::Johannesburg),
        "nairobi" | "kenya" | "eat" => Some(chrono_tz::Africa::Nairobi),
        "casablanca" | "morocco" => Some(chrono_tz::Africa::Casablanca),
        "accra" | "ghana" => Some(chrono_tz::Africa::Accra),

        _ => {
            // Try IANA timezone: "America/New_York"
            raw.trim().parse::<Tz>().ok()
        }
    }
}

fn local_timezone() -> Option<Tz> {
    let tz_str = iana_time_zone::get_timezone().ok()?;
    tz_str.parse::<Tz>().ok()
}

fn friendly_tz_name(tz: &Tz) -> String {
    match *tz {
        Tz::America__New_York => "EST".into(),
        Tz::America__Chicago => "CST".into(),
        Tz::America__Denver => "MST".into(),
        Tz::America__Los_Angeles => "PST".into(),
        Tz::Pacific__Honolulu => "HST".into(),
        Tz::America__Anchorage => "AKST".into(),
        Tz::America__Toronto => "Toronto".into(),
        Tz::America__Vancouver => "Vancouver".into(),
        Tz::Europe__London => "London".into(),
        Tz::Europe__Paris => "Paris".into(),
        Tz::Europe__Moscow => "Moscow".into(),
        Tz::Europe__Istanbul => "Istanbul".into(),
        Tz::Asia__Dubai => "Dubai".into(),
        Tz::Asia__Kolkata => "India".into(),
        Tz::Asia__Tokyo => "Tokyo".into(),
        Tz::Asia__Seoul => "Seoul".into(),
        Tz::Asia__Shanghai => "Shanghai".into(),
        Tz::Asia__Singapore => "Singapore".into(),
        Tz::Asia__Bangkok => "Bangkok".into(),
        Tz::Australia__Sydney => "Sydney".into(),
        Tz::Pacific__Auckland => "Auckland".into(),
        Tz::Africa__Cairo => "Cairo".into(),
        Tz::Africa__Lagos => "Lagos".into(),
        Tz::America__Sao_Paulo => "São Paulo".into(),
        Tz::Asia__Jerusalem => "Israel".into(),
        _ => format!("{}", tz),
    }
}

fn try_timezone_conversion(query: &str) -> Option<(String, String)> {
    // Try both strict and natural patterns
    let caps = TIME_RE.captures(query)
        .or_else(|| TIME_NATURAL_RE.captures(query))?;

    let hour: u32 = caps[1].parse().ok()?;
    let minute: u32 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
    let ampm = caps.get(3).map(|m| m.as_str().to_lowercase());
    let from_raw = &caps[4];
    let to_raw = &caps[5];

    let from_tz = lookup_timezone(from_raw)?;
    let to_tz = lookup_timezone(to_raw)?;

    let hour_24 = match ampm.as_deref() {
        Some("am") => if hour == 12 { 0 } else { hour },
        Some("pm") => if hour == 12 { 12 } else { hour + 12 },
        None | Some(_) => hour,
    };

    if hour_24 >= 24 || minute >= 60 { return None; }

    let time = NaiveTime::from_hms_opt(hour_24, minute, 0)?;
    let today = Local::now().date_naive();
    let naive_dt = today.and_time(time);

    let from_dt = naive_dt.and_local_timezone(from_tz).earliest()?;
    let to_dt: chrono::DateTime<Tz> = from_dt.with_timezone(&to_tz);

    let from_label = friendly_tz_name(&from_tz);
    let to_label = friendly_tz_name(&to_tz);

    let from_fmt = format_time_12h(hour_24, minute);
    let to_fmt = format_time_12h(to_dt.hour(), to_dt.minute());

    let day_diff = to_dt.date_naive().signed_duration_since(today).num_days();
    let day_note = match day_diff {
        0 => String::new(),
        1 => " (+1 day)".into(),
        -1 => " (-1 day)".into(),
        n => format!(" ({:+} days)", n),
    };

    let display = format!("{} {} = {} {}{}", from_fmt, from_label, to_fmt, to_label, day_note);
    let copy_val = format!("{} {}{}", to_fmt, to_label, day_note);
    Some((display, copy_val))
}

fn format_time_12h(hour24: u32, minute: u32) -> String {
    let (h, period) = if hour24 == 0 {
        (12, "AM")
    } else if hour24 < 12 {
        (hour24, "AM")
    } else if hour24 == 12 {
        (12, "PM")
    } else {
        (hour24 - 12, "PM")
    };
    if minute == 0 {
        format!("{}:00 {}", h, period)
    } else {
        format!("{}:{:02} {}", h, minute, period)
    }
}

// ---------------------------------------------------------------------------
// Number formatting
// ---------------------------------------------------------------------------

fn format_number(val: f64) -> String {
    if val.fract() == 0.0 && val.abs() < 1e15 {
        format!("{}", val as i64)
    } else {
        let s = format!("{:.6}", val);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CommandProvider for CalculatorProvider {
    fn name(&self) -> &str { "Calculator" }

    async fn commands(&self) -> Vec<CommandEntry> { vec![] }

    fn is_dynamic(&self) -> bool { true }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let trimmed = query.trim();
        if trimmed.is_empty() { return vec![]; }

        // Normalize input for flexible matching
        let normalized = normalize_query(trimmed);

        // Try unit conversion: original first (for natural patterns), then normalized (for strict)
        if let Some((display, _)) = try_unit_conversion(trimmed)
            .or_else(|| try_unit_conversion(&normalized))
        {
            return vec![CommandEntry {
                id: format!("calc.unit.{}", trimmed),
                name: display,
                description: "Press Enter to copy result".into(),
                category: "Calculator".into(),
                icon: None,
                match_indices: vec![],
                score: 100,
            }];
        }

        // Try timezone conversion: original first, then normalized
        if let Some((display, _)) = try_timezone_conversion(trimmed)
            .or_else(|| try_timezone_conversion(&normalized))
        {
            return vec![CommandEntry {
                id: format!("calc.tz.{}", trimmed),
                name: display,
                description: "Press Enter to copy result".into(),
                category: "Calculator".into(),
                icon: None,
                match_indices: vec![],
                score: 100,
            }];
        }

        // Fall back to math expression
        if !trimmed.chars().any(|c| c.is_ascii_digit()) { return vec![]; }

        match meval::eval_str(trimmed) {
            Ok(result) => {
                let formatted = format_number(result);
                vec![CommandEntry {
                    id: format!("calc.{}", trimmed),
                    name: format!("{} = {}", trimmed, formatted),
                    description: "Press Enter to copy result".into(),
                    category: "Calculator".into(),
                    icon: None,
                    match_indices: vec![],
                    score: 100,
                }]
            }
            Err(_) => vec![],
        }
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        let copy_value = if let Some(expr) = id.strip_prefix("calc.unit.") {
            let normalized = normalize_query(expr);
            try_unit_conversion(expr)
                .or_else(|| try_unit_conversion(&normalized))
                .map(|(_, v)| v)
        } else if let Some(expr) = id.strip_prefix("calc.tz.") {
            let normalized = normalize_query(expr);
            try_timezone_conversion(expr)
                .or_else(|| try_timezone_conversion(&normalized))
                .map(|(_, v)| v)
        } else if let Some(expr) = id.strip_prefix("calc.") {
            match meval::eval_str(expr) {
                Ok(result) => Some(format_number(result)),
                Err(e) => return Some(Err(e.to_string())),
            }
        } else {
            return None;
        };

        match copy_value {
            Some(val) => Some(
                std::process::Command::new("sh")
                    .args(["-c", &format!("printf '%s' '{}' | pbcopy", val)])
                    .spawn()
                    .map(|_| format!("Copied {}", val))
                    .map_err(|e| e.to_string()),
            ),
            None => Some(Err("Failed to evaluate expression".into())),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unit conversion tests --

    #[test]
    fn test_unit_pint_to_cups() {
        let (display, val) = try_unit_conversion("1 pint to cups").unwrap();
        assert!(display.contains("2"), "1 pint should be ~2 cups, got: {}", display);
        assert!(val.parse::<f64>().unwrap() > 1.9);
    }

    #[test]
    fn test_unit_kg_to_lbs() {
        let (_, val) = try_unit_conversion("5 kg to lbs").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 11.0231).abs() < 0.01);
    }

    #[test]
    fn test_unit_km_to_miles() {
        let (_, val) = try_unit_conversion("10 km to miles").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 6.21371).abs() < 0.01);
    }

    #[test]
    fn test_unit_celsius_to_fahrenheit() {
        let (_, val) = try_unit_conversion("100 c to f").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 212.0).abs() < 0.01);
    }

    #[test]
    fn test_unit_fahrenheit_to_celsius() {
        let (_, val) = try_unit_conversion("32 f to c").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!(v.abs() < 0.01);
    }

    #[test]
    fn test_unit_gb_to_mb() {
        let (_, val) = try_unit_conversion("2 gb to mb").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 2000.0).abs() < 0.01);
    }

    #[test]
    fn test_unit_cups_to_ml() {
        let (_, val) = try_unit_conversion("1 cup to ml").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 236.588).abs() < 1.0);
    }

    #[test]
    fn test_unit_hours_to_minutes() {
        let (_, val) = try_unit_conversion("2.5 hours to minutes").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_unit_mph_to_kph() {
        let (_, val) = try_unit_conversion("60 mph to kph").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 96.5606).abs() < 0.1);
    }

    #[test]
    fn test_unit_incompatible_kinds() {
        assert!(try_unit_conversion("5 kg to miles").is_none());
    }

    #[test]
    fn test_unit_commas_in_number() {
        let (_, val) = try_unit_conversion("1,000 mm to m").unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 1.0).abs() < 0.01);
    }

    // -- Natural language unit tests --

    #[test]
    fn test_natural_how_many_cups_in_pint() {
        let orig = "how many cups in a pint";
        let result = try_unit_conversion(orig);
        assert!(result.is_some(), "Should parse 'how many cups in a pint'");
        let (_, val) = result.unwrap();
        let v: f64 = val.parse().unwrap();
        assert!((v - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_natural_convert_kg() {
        let orig = "convert 10 kg to pounds";
        let result = try_unit_conversion(orig)
            .or_else(|| try_unit_conversion(&normalize_query(orig)));
        assert!(result.is_some(), "Should parse 'convert 10 kg to pounds'");
    }

    // -- Timezone tests --

    #[test]
    fn test_timezone_est_to_dubai() {
        let result = try_timezone_conversion("10pm est in dubai");
        assert!(result.is_some(), "Should parse '10pm est in dubai'");
        let (display, _) = result.unwrap();
        // EST maps to America/New_York (may be EDT during DST)
        assert!(display.contains("AM") && display.contains("Dubai"),
            "Should show morning in Dubai, got: {}", display);
    }

    #[test]
    fn test_timezone_with_minutes() {
        let result = try_timezone_conversion("3:30pm pst in tokyo");
        assert!(result.is_some());
        let (display, _) = result.unwrap();
        assert!(display.contains("AM") && display.contains("Tokyo"),
            "Should convert to Tokyo time, got: {}", display);
    }

    #[test]
    fn test_timezone_natural_what_time() {
        let q = normalize_query("what time is 5am est in dubai");
        let result = try_timezone_conversion(&q);
        assert!(result.is_some(), "Should parse 'what time is 5am est in dubai', normalized to: {}", q);
        let (display, _) = result.unwrap();
        assert!(display.contains("PM") && display.contains("Dubai"),
            "5am EST should be afternoon in Dubai, got: {}", display);
    }

    #[test]
    fn test_timezone_typo_waht() {
        let q = normalize_query("waht time is 5am est in dubai");
        let result = try_timezone_conversion(&q);
        assert!(result.is_some(), "Should handle typo 'waht time is'");
    }

    #[test]
    fn test_timezone_my_time() {
        // "my time" should resolve to local system timezone
        let result = lookup_timezone("my time");
        assert!(result.is_some(), "'my time' should resolve to local timezone");
    }

    // -- Math tests --

    #[test]
    fn test_math_still_works() {
        let val = format_number(meval::eval_str("2+2").unwrap());
        assert_eq!(val, "4");
    }

    #[test]
    fn test_format_number_integer() {
        assert_eq!(format_number(42.0), "42");
    }

    #[test]
    fn test_format_number_decimal() {
        assert_eq!(format_number(3.14), "3.14");
    }
}
