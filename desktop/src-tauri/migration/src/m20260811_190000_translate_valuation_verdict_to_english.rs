use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// The `verdict` column used to store the Rust `Verdict` enum's Portuguese
// labels ("BARATO"/"CARO") — `domain::*::Verdict::as_str()` now returns
// "CHEAP"/"EXPENSIVE" instead (repo is public on GitHub, UI-visible strings
// must be English per project guidelines). Existing rows saved under the
// old labels need backfilling too, or `VerdictBadge`'s `verdict === "CHEAP"`
// check (and the Analysis screen's live-price comparison) would silently
// mis-color every valuation saved before this migration.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("UPDATE valuation SET verdict = 'CHEAP' WHERE verdict = 'BARATO'")
            .await?;
        db.execute_unprepared("UPDATE valuation SET verdict = 'EXPENSIVE' WHERE verdict = 'CARO'")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("UPDATE valuation SET verdict = 'BARATO' WHERE verdict = 'CHEAP'")
            .await?;
        db.execute_unprepared("UPDATE valuation SET verdict = 'CARO' WHERE verdict = 'EXPENSIVE'")
            .await?;
        Ok(())
    }
}
