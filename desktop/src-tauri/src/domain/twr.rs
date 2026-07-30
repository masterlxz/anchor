use crate::error::AppError;

pub struct CashFlow {
    pub amount: f64,
    pub weight: f64,
}

// R_mes = (EMV - BMV - CF_total) / (BMV + Σ(CF_i × W_i))
// Método Dietz Modificado (Fase 10.3) — `weight` de cada fluxo já vem
// calculado pelo chamador (dias restantes no mês / dias no mês), esta
// função só soma e divide.
pub fn modified_dietz(bmv: f64, emv: f64, cash_flows: &[CashFlow]) -> Result<f64, AppError> {
    let cf_total: f64 = cash_flows.iter().map(|cf| cf.amount).sum();
    let weighted_cf: f64 = cash_flows.iter().map(|cf| cf.amount * cf.weight).sum();
    let denominator = bmv + weighted_cf;

    if denominator == 0.0 {
        return Err(AppError::InvalidGuard(
            "Modified Dietz denominator is zero (BMV and weighted cash flows cancel out)"
                .to_string(),
        ));
    }

    Ok((emv - bmv - cf_total) / denominator)
}

// R_total = Π(1 + R_mes_i) − 1 — geometricamente componível.
pub fn chain(monthly_returns: &[f64]) -> f64 {
    monthly_returns.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_phase_md_worked_example_january() {
        let cash_flows = [CashFlow {
            amount: 2000.0,
            weight: 21.0 / 31.0,
        }];

        let r = modified_dietz(10000.0, 12500.0, &cash_flows).unwrap();

        assert!((r - 0.04403409090909091).abs() < 1e-9);
    }

    #[test]
    fn matches_phase_md_worked_example_february_no_flows() {
        let r = modified_dietz(12500.0, 12250.0, &[]).unwrap();

        assert!((r - (-0.02)).abs() < 1e-9);
    }

    #[test]
    fn chain_composes_january_and_february() {
        let total = chain(&[0.04403409090909091, -0.02]);

        assert!((total - 0.023153409090908905).abs() < 1e-9);
    }

    #[test]
    fn rejects_degenerate_zero_denominator() {
        assert!(matches!(
            modified_dietz(0.0, 0.0, &[]),
            Err(AppError::InvalidGuard(_))
        ));
    }
}
