// Fase 13.6 (parte "futuro") — resolve as linhas que a extração via IA de um
// "Relatório Proventos" da CVM devolveu pra um único valor aplicável ao
// portfolio. Função pura, mesmo espírito de `domain::dividend_suggestion`:
// o comando (`commands::cvm_dividend_notice`) cuida do PDF/IA/banco, isto
// aqui só decide "dá pra confiar nesse resultado ou não".
//
// Decisão do dono do projeto: um documento pode listar mais de uma classe
// de ação (ON/PN/units), cada uma com seu próprio valor/cota — se os
// valores não baterem entre as linhas e não der pra casar o ISIN certo com
// o ticker do portfolio, a sugestão automática é pulada (`None`) em vez de
// arriscar um valor errado.
use serde::Deserialize;

use crate::domain::proventos::{PAYMENT_TYPE_DIVIDENDO, PAYMENT_TYPE_JSCP};

/// `Deserialize` aqui é só conveniência (evita uma struct duplicada +
/// conversão manual no lado do comando, que é quem de fato lida com IA/rede)
/// — o parse em si continua puro, sem I/O.
#[derive(Deserialize)]
pub struct ExtractedNotice {
    pub label: Option<String>,
    pub rate_per_share: Option<f64>,
    pub payment_date: Option<String>,
    pub com_date: Option<String>,
}

pub struct ResolvedNotice {
    pub payment_date: String,
    pub com_date: Option<String>,
    pub rate_per_share: f64,
    pub payment_type: String,
}

pub fn resolve_notices(notices: Vec<ExtractedNotice>) -> Option<ResolvedNotice> {
    let usable: Vec<ExtractedNotice> = notices
        .into_iter()
        .filter(|n| n.rate_per_share.is_some() && n.payment_date.is_some() && n.label.is_some())
        .collect();
    let first = usable.first()?;
    let all_agree = usable.iter().all(|n| {
        n.rate_per_share == first.rate_per_share
            && n.payment_date == first.payment_date
            && n.label == first.label
    });
    if !all_agree {
        return None;
    }

    let payment_type = if first.label.as_deref() == Some("JCP") {
        PAYMENT_TYPE_JSCP
    } else {
        PAYMENT_TYPE_DIVIDENDO
    }
    .to_string();

    Some(ResolvedNotice {
        payment_date: first.payment_date.clone().expect("filtered above"),
        com_date: first.com_date.clone(),
        rate_per_share: first.rate_per_share.expect("filtered above"),
        payment_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(label: &str, rate: f64, payment_date: &str, com_date: Option<&str>) -> ExtractedNotice {
        ExtractedNotice {
            label: Some(label.to_string()),
            rate_per_share: Some(rate),
            payment_date: Some(payment_date.to_string()),
            com_date: com_date.map(str::to_string),
        }
    }

    #[test]
    fn single_usable_row_resolves() {
        let resolved = resolve_notices(vec![notice("DIVIDENDO", 0.5, "2026-03-05", Some("2026-02-10"))])
            .unwrap();
        assert_eq!(resolved.payment_date, "2026-03-05");
        assert_eq!(resolved.com_date, Some("2026-02-10".to_string()));
        assert_eq!(resolved.rate_per_share, 0.5);
        assert_eq!(resolved.payment_type, PAYMENT_TYPE_DIVIDENDO);
    }

    #[test]
    fn jcp_label_maps_to_jscp_payment_type() {
        let resolved = resolve_notices(vec![notice("JCP", 0.3, "2026-03-05", None)]).unwrap();
        assert_eq!(resolved.payment_type, PAYMENT_TYPE_JSCP);
    }

    #[test]
    fn multiple_rows_agreeing_resolve_to_one_value() {
        // Caso real confirmado ao vivo (Banco do Brasil): 3 classes de ação
        // (ON/PN/units), mesmo valor/cota nas 3.
        let resolved = resolve_notices(vec![
            notice("DIVIDENDO", 0.2163042918, "2026-03-05", Some("2026-02-10")),
            notice("DIVIDENDO", 0.2163042918, "2026-03-05", Some("2026-02-10")),
            notice("DIVIDENDO", 0.2163042918, "2026-03-05", Some("2026-02-10")),
        ])
        .unwrap();
        assert_eq!(resolved.rate_per_share, 0.2163042918);
    }

    #[test]
    fn diverging_rows_never_resolve() {
        let result = resolve_notices(vec![
            notice("DIVIDENDO", 0.5, "2026-03-05", None),
            notice("DIVIDENDO", 0.7, "2026-03-05", None),
        ]);
        assert!(result.is_none());
    }

    #[test]
    fn diverging_payment_dates_never_resolve() {
        let result = resolve_notices(vec![
            notice("DIVIDENDO", 0.5, "2026-03-05", None),
            notice("DIVIDENDO", 0.5, "2026-03-06", None),
        ]);
        assert!(result.is_none());
    }

    #[test]
    fn rows_missing_essential_fields_are_discarded_before_agreement_check() {
        let incomplete = ExtractedNotice {
            label: None,
            rate_per_share: Some(0.5),
            payment_date: Some("2026-03-05".to_string()),
            com_date: None,
        };
        let resolved = resolve_notices(vec![
            incomplete,
            notice("DIVIDENDO", 0.5, "2026-03-05", None),
        ])
        .unwrap();
        assert_eq!(resolved.rate_per_share, 0.5);
    }

    #[test]
    fn no_usable_rows_returns_none() {
        assert!(resolve_notices(vec![]).is_none());
    }
}
