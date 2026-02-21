pub fn lookup_constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "tau" => Some(std::f64::consts::TAU),
        "golden_ratio" => Some(1.618_033_988_749_895),
        "euler_mascheroni" => Some(0.577_215_664_901_532_9),
        "ln2" => Some(std::f64::consts::LN_2),
        "ln10" => Some(std::f64::consts::LN_10),
        "sqrt2" => Some(std::f64::consts::SQRT_2),
        _ => None,
    }
}
