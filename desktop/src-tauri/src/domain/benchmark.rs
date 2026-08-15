// Fase 13.5 — retorno mensal de um benchmark que só tem preço/cotação (IBOV
// via `^BVSP`, IVVB11 via ticker normal), a partir de `stock_price_history`.
// CDI/IPCA não passam por aqui — já chegam como % mensal pronto da BCB
// (`macro_index_monthly`), sem preço nem cálculo nenhum.

use chrono::{Datelike, NaiveDate};

pub struct PricePoint {
    pub date: NaiveDate,
    pub close: f64,
}

pub struct MonthlyBenchmarkReturn {
    pub year_month: String,
    pub r_month_pct: f64,
}

// `month_ends` precisa de N+1 datas ascendentes pra gerar N retornos — a
// primeira só serve de base (fim do mês anterior ao primeiro mês reportado),
// sem retorno próprio, mesma lógica de BMV precisar do mês anterior em
// `commands::profitability`. Pra cada par consecutivo, pega o preço mais
// próximo dentro de `tolerance_days` (mesma ideia de
// `commands::profitability::closest_price`, mas pura/síncrona sobre um
// slice, reutilizável e testável). Mês sem candidato dentro da tolerância é
// PULADO, não falha o resto — ao contrário de `closest_price` (que falha
// alto porque preço faltando de um ativo que o usuário possui é bug de
// verdade), aqui um benchmark sem backfill ainda, ou com o mês corrente
// ainda não fechado/publicado, é estado normal, não erro.
pub fn monthly_returns_from_prices(
    prices: &[PricePoint],
    month_ends: &[NaiveDate],
    tolerance_days: i64,
) -> Vec<MonthlyBenchmarkReturn> {
    let closest = |target: NaiveDate| -> Option<f64> {
        prices
            .iter()
            .filter(|p| (p.date - target).num_days().abs() <= tolerance_days)
            .min_by_key(|p| (p.date - target).num_days().abs())
            .map(|p| p.close)
    };

    let mut results = Vec::new();
    for window in month_ends.windows(2) {
        let [prev, curr] = window else { continue };
        let (Some(prev_close), Some(curr_close)) = (closest(*prev), closest(*curr)) else {
            continue;
        };
        results.push(MonthlyBenchmarkReturn {
            year_month: format!("{:04}-{:02}", curr.year(), curr.month()),
            r_month_pct: (curr_close / prev_close - 1.0) * 100.0,
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn point(y: i32, m: u32, d: u32, close: f64) -> PricePoint {
        PricePoint { date: date(y, m, d), close }
    }

    #[test]
    fn exact_dates_compute_correct_percentages() {
        let prices = [point(2026, 1, 31, 100.0), point(2026, 2, 28, 110.0), point(2026, 3, 31, 99.0)];
        let month_ends = [date(2026, 1, 31), date(2026, 2, 28), date(2026, 3, 31)];

        let result = monthly_returns_from_prices(&prices, &month_ends, 7);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].year_month, "2026-02");
        assert!((result[0].r_month_pct - 10.0).abs() < 1e-9);
        assert_eq!(result[1].year_month, "2026-03");
        assert!((result[1].r_month_pct - (-10.0)).abs() < 1e-9);
    }

    #[test]
    fn picks_closest_price_within_tolerance() {
        // Fim de mês cai num fim de semana — o pregão mais próximo (dentro
        // da tolerância) é escolhido, não o exato.
        let prices = [point(2026, 1, 30, 100.0), point(2026, 2, 27, 105.0)];
        let month_ends = [date(2026, 1, 31), date(2026, 2, 28)];

        let result = monthly_returns_from_prices(&prices, &month_ends, 7);

        assert_eq!(result.len(), 1);
        assert!((result[0].r_month_pct - 5.0).abs() < 1e-9);
    }

    #[test]
    fn month_without_candidate_within_tolerance_is_skipped_but_others_survive() {
        // Fevereiro não tem preço nenhum perto (tolerância 7 dias) — os
        // pares que dependem dele (dez->jan tá ok, jan->fev e fev->mar são
        // pulados), mas dez->jan continua íntegro. Mesmo espírito do "mês
        // corrente ainda não publicado" que motivou esse desenho: um buraco
        // no meio não deve derrubar o resto da série.
        let prices = [point(2025, 12, 31, 100.0), point(2026, 1, 31, 105.0), point(2026, 3, 31, 90.0)];
        let month_ends = [date(2025, 12, 31), date(2026, 1, 31), date(2026, 2, 28), date(2026, 3, 31)];

        let result = monthly_returns_from_prices(&prices, &month_ends, 7);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].year_month, "2026-01");
        assert!((result[0].r_month_pct - 5.0).abs() < 1e-9);
    }

    #[test]
    fn single_month_end_has_no_baseline_and_returns_empty() {
        let prices = [point(2026, 1, 31, 100.0)];
        let month_ends = [date(2026, 1, 31)];

        let result = monthly_returns_from_prices(&prices, &month_ends, 7);

        assert!(result.is_empty());
    }
}
