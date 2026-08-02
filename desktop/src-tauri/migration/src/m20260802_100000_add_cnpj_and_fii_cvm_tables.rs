use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(string_null(Assets::Cnpj))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FiiCvmMonthly::Table)
                    .if_not_exists()
                    .col(pk_auto(FiiCvmMonthly::Id))
                    .col(string(FiiCvmMonthly::Cnpj))
                    .col(string(FiiCvmMonthly::ReferenceDate))
                    .col(double(FiiCvmMonthly::PatrimonioLiquido))
                    .col(double(FiiCvmMonthly::ValorPatrimonialCota))
                    .col(integer_null(FiiCvmMonthly::NumeroCotistas))
                    .col(double_null(FiiCvmMonthly::DividendYieldMes))
                    .col(double_null(FiiCvmMonthly::RentabilidadeEfetivaMes))
                    .col(string(FiiCvmMonthly::Source))
                    .col(string(FiiCvmMonthly::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_fii_cvm_monthly_cnpj_date")
                    .table(FiiCvmMonthly::Table)
                    .col(FiiCvmMonthly::Cnpj)
                    .col(FiiCvmMonthly::ReferenceDate)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FiiCvmProperties::Table)
                    .if_not_exists()
                    .col(pk_auto(FiiCvmProperties::Id))
                    .col(string(FiiCvmProperties::Cnpj))
                    .col(string(FiiCvmProperties::ReferenceDate))
                    .col(string(FiiCvmProperties::NomeImovel))
                    .col(string_null(FiiCvmProperties::Endereco))
                    .col(double_null(FiiCvmProperties::AreaM2))
                    .col(double_null(FiiCvmProperties::PercentualVacancia))
                    .col(double_null(FiiCvmProperties::PercentualInadimplencia))
                    .col(double_null(FiiCvmProperties::PercentualReceitasFii))
                    .col(double_null(FiiCvmProperties::PercentualLocado))
                    .col(string(FiiCvmProperties::Source))
                    .col(string(FiiCvmProperties::FetchedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_fii_cvm_properties_cnpj_date_imovel")
                    .table(FiiCvmProperties::Table)
                    .col(FiiCvmProperties::Cnpj)
                    .col(FiiCvmProperties::ReferenceDate)
                    .col(FiiCvmProperties::NomeImovel)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FiiCvmProperties::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FiiCvmMonthly::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::Cnpj)
                    .to_owned(),
            )
            .await
    }
}

/// Fase 10, item 8, Sessão 41 — indicadores de FII direto da CVM (dados
/// abertos, `dados.cvm.gov.br/dados/FII/DOC/`), priorizados sobre a bolsai
/// por decisão explícita do dono do projeto ("quero usar a CVM pro máximo
/// de coisa possível"). `assets.cnpj` (nullable — só relevante pra `fii`
/// por ora) é resolvido uma única vez no cadastro (`commands/fii.rs::
/// resolve_fii_cnpj`, bolsai só como passo auxiliar de ticker→CNPJ, nunca
/// mais chamada depois disso); a partir daí toda leitura recorrente usa só
/// estas duas tabelas, alimentadas direto da CVM.
///
/// `FiiCvmMonthly` — informe mensal (`INF_MENSAL/.../complemento`):
/// patrimônio líquido, valor patrimonial da cota (NAV — não é preço de
/// mercado, esse continua vindo do Yahoo), nº de cotistas, dividend yield e
/// rentabilidade efetiva do mês. Índice único `(cnpj, reference_date)` pro
/// mesmo padrão `INSERT OR IGNORE` de `stock_price_history` — atualizar de
/// novo só acrescenta o mês novo, não duplica.
///
/// `FiiCvmProperties` — informe trimestral, arquivo `imovel`
/// (`INF_TRIMESTRAL/.../imovel`): um item por imóvel de cada fundo, com
/// vacância e inadimplência **por imóvel** — exatamente o indicador pedido
/// pelo dono do projeto, direto da fonte oficial. Índice único `(cnpj,
/// reference_date, nome_imovel)` — o nome do imóvel distingue as várias
/// linhas de um mesmo fundo no mesmo trimestre.
#[derive(DeriveIden)]
enum Assets {
    Table,
    Cnpj,
}

#[derive(DeriveIden)]
enum FiiCvmMonthly {
    Table,
    Id,
    Cnpj,
    ReferenceDate,
    PatrimonioLiquido,
    ValorPatrimonialCota,
    NumeroCotistas,
    DividendYieldMes,
    RentabilidadeEfetivaMes,
    Source,
    FetchedAt,
}

#[derive(DeriveIden)]
enum FiiCvmProperties {
    Table,
    Id,
    Cnpj,
    ReferenceDate,
    NomeImovel,
    Endereco,
    AreaM2,
    PercentualVacancia,
    PercentualInadimplencia,
    PercentualReceitasFii,
    PercentualLocado,
    Source,
    FetchedAt,
}
