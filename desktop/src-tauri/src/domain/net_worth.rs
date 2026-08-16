/// Divisão-guarda pura — em ponto flutuante `x/0.0` não panica, vira
/// `inf`/`NaN` silenciosamente, que vazaria pro JSON do frontend sem aviso.
/// Mesma granularidade de "guard de uma linha ainda ganha função + teste"
/// de `domain::position_pricing`/`domain::liability`.
pub fn compute_leverage(total_debt: f64, net_worth: f64) -> Option<f64> {
    if net_worth <= 0.0 {
        return None;
    }
    Some(total_debt / net_worth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_leverage_guards_zero_net_worth() {
        assert_eq!(compute_leverage(500.0, 0.0), None);
    }

    #[test]
    fn compute_leverage_guards_negative_net_worth() {
        assert_eq!(compute_leverage(500.0, -100.0), None);
    }

    #[test]
    fn compute_leverage_divides_debt_by_net_worth() {
        assert_eq!(compute_leverage(500.0, 2000.0), Some(0.25));
    }
}
