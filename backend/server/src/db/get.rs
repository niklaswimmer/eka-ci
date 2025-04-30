use futures::stream::BoxStream;
use sqlx::SqlitePool;

use super::model::build::{DrvBuildEvent, DrvBuildState, DrvId};

pub async fn get_latest_build_event(
    derivation: &DrvId,
    pool: &SqlitePool,
) -> anyhow::Result<Option<DrvBuildEvent>> {
    let event = sqlx::query_as(
        r#"
SELECT MAX(rowid), derivation, build_attempt, state, timestamp
FROM DrvBuildEvent
WHERE derivation = ?
        "#,
    )
    .bind(derivation)
    .fetch_optional(pool)
    .await?;

    Ok(event)
}

pub fn get_derivations_in_state<'a>(
    state: DrvBuildState,
    pool: &'a SqlitePool,
) -> BoxStream<'a, sqlx::Result<DrvBuildEvent>> {
    sqlx::query_as(
        r#"
SELECT MAX(rowid), derivation, build_attempt, state, timestamp
FROM DrvBuildEvent
GROUP BY derivation
HAVING state = ?
        "#,
    )
    .bind(state)
    .fetch(pool)
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use futures::StreamExt;
    use sqlx::SqlitePool;

    use crate::db::{get::get_latest_build_event, model::build::{DrvBuildState, DrvId}};
    use super::get_derivations_in_state;

    #[sqlx::test(migrations = "./sql/migrations")]
    async fn select_latest_state(pool: SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO DrvBuildEvent
    (derivation, build_attempt, state)
VALUES
(?1, 1, ?2),
(?1, 1, ?3),
(?1, 2, ?4),
(?1, 2, ?5)
            "#
        )
        .bind(DrvId::dummy())
        .bind(DrvBuildState::Queued)
        .bind(DrvBuildState::Buildable)
        .bind(DrvBuildState::Building)
        .bind(DrvBuildState::Blocked)
        .execute(&pool)
        .await?;

        let Some(result) = get_latest_build_event(&DrvId::dummy(), &pool).await? else {
            bail!("Expected query to find a result")
        };

        assert_eq!(result.state, DrvBuildState::Blocked);

        Ok(())
    }

    #[ignore = "Need to use valid derivations in sample data, otherwise stuff fails"]
    #[sqlx::test(migrations = "./sql/migrations")]
    async fn select_multiple_derivations_in_state(pool: SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO DrvBuildEvent
    (derivation, build_attempt, state)
VALUES
('drv1', 1, ?1),
('drv1', 1, ?2),
('drv1', 1, ?3),
('drv2', 1, ?3),
('drv2', 1, ?4),
('drv3', 1, ?3)
            "#,
        )
        .bind(DrvBuildState::Queued)
        .bind(DrvBuildState::Buildable)
        .bind(DrvBuildState::Building)
        .bind(DrvBuildState::Blocked)
        .execute(&pool)
        .await?;

        let result: Vec<_> = get_derivations_in_state(DrvBuildState::Building, &pool)
            .collect()
            .await;

        dbg!(result);

        assert!(false);

        Ok(())
    }
}
