use crate::error::AppError;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AmortizationSystem {
    Price,
    Sac,
}

impl AmortizationSystem {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "price" => Ok(Self::Price),
            "sac" => Ok(Self::Sac),
            other => Err(AppError::InvalidGuard(format!(
                "sistema_amortizacao '{other}' inválido (esperado 'price' ou 'sac')"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Installment {
    pub numero_parcela: i32,
    pub valor_parcela: f64,
    pub valor_juros: f64,
    pub valor_amortizacao: f64,
    pub saldo_devedor_apos: f64,
}

/// Gera o cronograma completo de amortização — Price (parcela fixa) ou SAC
/// (amortização fixa, parcela decrescente). `taxa_mensal` é decimal (ex.
/// 0.01 pra 1% a.m.), nunca percentual bruto (quem chama já divide por 100).
pub fn compute_installments(
    principal: f64,
    taxa_mensal: f64,
    prazo_meses: i32,
    sistema: AmortizationSystem,
) -> Result<Vec<Installment>, AppError> {
    if principal <= 0.0 {
        return Err(AppError::InvalidGuard(
            "principal deve ser maior que zero".to_string(),
        ));
    }
    if taxa_mensal < 0.0 {
        return Err(AppError::InvalidGuard(
            "taxa mensal não pode ser negativa".to_string(),
        ));
    }
    if prazo_meses <= 0 {
        return Err(AppError::InvalidGuard(
            "prazo (em meses) deve ser maior que zero".to_string(),
        ));
    }

    let n = prazo_meses;
    let i = taxa_mensal;

    let mut installments = Vec::with_capacity(n as usize);
    let mut saldo = principal;

    match sistema {
        AmortizationSystem::Price => {
            // Parcela fixa. Quando i == 0 a fórmula usual degenera (divisão
            // por zero em (1+i)^n - 1) — cai pro caso trivial de dívida sem
            // juros, parcela = principal / n.
            let pmt = if i == 0.0 {
                principal / f64::from(n)
            } else {
                let factor = (1.0 + i).powi(n);
                principal * (i * factor) / (factor - 1.0)
            };

            for numero in 1..=n {
                let juros = saldo * i;
                let mut amortizacao = pmt - juros;
                // Última parcela absorve o resíduo de arredondamento —
                // garante saldo final exatamente zero.
                if numero == n {
                    amortizacao = saldo;
                }
                saldo -= amortizacao;
                installments.push(Installment {
                    numero_parcela: numero,
                    valor_parcela: amortizacao + juros,
                    valor_juros: juros,
                    valor_amortizacao: amortizacao,
                    saldo_devedor_apos: saldo,
                });
            }
        }
        AmortizationSystem::Sac => {
            let amortizacao_fixa = principal / f64::from(n);

            for numero in 1..=n {
                let juros = saldo * i;
                let amortizacao = if numero == n { saldo } else { amortizacao_fixa };
                saldo -= amortizacao;
                installments.push(Installment {
                    numero_parcela: numero,
                    valor_parcela: amortizacao + juros,
                    valor_juros: juros,
                    valor_amortizacao: amortizacao,
                    saldo_devedor_apos: saldo,
                });
            }
        }
    }

    Ok(installments)
}

/// Anualiza uma taxa mensal por capitalização composta — mesma matemática
/// que `compute_installments` já aplica internamente (juros = saldo ×
/// `taxa_mensal`, todo mês). `taxa_mensal` é decimal (0.03 pra 3% a.m.),
/// mesma convenção de `compute_installments`. Usado pra comparar o custo de
/// uma dívida com o retorno anualizado do ativo vinculado (Fase 12).
pub fn annualize_monthly_rate(taxa_mensal: f64) -> f64 {
    (1.0 + taxa_mensal).powi(12) - 1.0
}

/// Saldo devedor "hoje" — principal menos a amortização de toda parcela já
/// vencida (`data_vencimento <= hoje`, comparação lexicográfica de string
/// `YYYY-MM-DD`, mesmo formato usado em todo o resto do projeto). Parcelas
/// futuras não contam — a dívida só "diminui" quando a parcela
/// efetivamente vence. Função pura, sem I/O, pro resumo de dívida total
/// (`list_liabilities`).
pub fn compute_outstanding_balance(
    principal: f64,
    installments: &[(&str, f64)],
    hoje: &str,
) -> f64 {
    let amortizado: f64 = installments
        .iter()
        .filter(|(data_vencimento, _)| *data_vencimento <= hoje)
        .map(|(_, valor_amortizacao)| valor_amortizacao)
        .sum();
    principal - amortizado
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_installments_rejects_non_positive_principal() {
        assert!(compute_installments(0.0, 0.01, 12, AmortizationSystem::Price).is_err());
        assert!(compute_installments(-100.0, 0.01, 12, AmortizationSystem::Price).is_err());
    }

    #[test]
    fn compute_installments_rejects_negative_rate() {
        assert!(compute_installments(1000.0, -0.01, 12, AmortizationSystem::Price).is_err());
    }

    #[test]
    fn compute_installments_rejects_non_positive_term() {
        assert!(compute_installments(1000.0, 0.01, 0, AmortizationSystem::Price).is_err());
    }

    #[test]
    fn price_installments_have_constant_value() {
        let installments =
            compute_installments(10_000.0, 0.02, 12, AmortizationSystem::Price).unwrap();
        let first = installments[0].valor_parcela;
        for installment in &installments {
            assert!(
                (installment.valor_parcela - first).abs() < 0.01,
                "parcela {} difere da primeira: {} vs {}",
                installment.numero_parcela,
                installment.valor_parcela,
                first
            );
        }
    }

    #[test]
    fn price_amortization_sums_to_principal() {
        let principal = 10_000.0;
        let installments =
            compute_installments(principal, 0.015, 24, AmortizationSystem::Price).unwrap();
        let total_amortizacao: f64 = installments.iter().map(|i| i.valor_amortizacao).sum();
        assert!((total_amortizacao - principal).abs() < 0.01);
        assert!(installments.last().unwrap().saldo_devedor_apos.abs() < 0.001);
    }

    #[test]
    fn sac_amortization_is_constant_and_sums_to_principal() {
        let principal = 12_000.0;
        let installments =
            compute_installments(principal, 0.01, 12, AmortizationSystem::Sac).unwrap();
        let first = installments[0].valor_amortizacao;
        for installment in &installments {
            assert!((installment.valor_amortizacao - first).abs() < 0.01);
        }
        let total_amortizacao: f64 = installments.iter().map(|i| i.valor_amortizacao).sum();
        assert!((total_amortizacao - principal).abs() < 0.01);
        assert!(installments.last().unwrap().saldo_devedor_apos.abs() < 0.001);
    }

    #[test]
    fn sac_installments_decrease_over_time() {
        let installments =
            compute_installments(10_000.0, 0.02, 10, AmortizationSystem::Sac).unwrap();
        for pair in installments.windows(2) {
            assert!(pair[1].valor_parcela < pair[0].valor_parcela);
        }
    }

    #[test]
    fn zero_rate_price_splits_principal_evenly_with_no_interest() {
        let installments =
            compute_installments(1_200.0, 0.0, 12, AmortizationSystem::Price).unwrap();
        for installment in &installments {
            assert_eq!(installment.valor_juros, 0.0);
            assert!((installment.valor_parcela - 100.0).abs() < 0.001);
        }
    }

    #[test]
    fn compute_outstanding_balance_ignores_future_installments() {
        let installments = vec![("2026-01-01", 100.0), ("2026-06-01", 100.0)];
        let balance = compute_outstanding_balance(1000.0, &installments, "2026-03-01");
        assert_eq!(balance, 900.0);
    }

    #[test]
    fn compute_outstanding_balance_counts_installment_due_today() {
        let installments = vec![("2026-03-01", 100.0)];
        let balance = compute_outstanding_balance(1000.0, &installments, "2026-03-01");
        assert_eq!(balance, 900.0);
    }

    #[test]
    fn annualize_monthly_rate_of_zero_is_zero() {
        assert_eq!(annualize_monthly_rate(0.0), 0.0);
    }

    #[test]
    fn annualize_monthly_rate_compounds_over_twelve_months() {
        // 1% a.m. -> (1.01)^12 - 1 ~= 12.6825%
        let annual = annualize_monthly_rate(0.01);
        assert!((annual - 0.12682503013196977).abs() < 1e-9);
    }
}
